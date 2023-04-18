use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

use cgmath::prelude::*;
use encase::ShaderType;
use rayon::prelude::*;

#[derive(Clone, Copy, ShaderType)]
pub struct Particle {
    pub position: cgmath::Vector3<f32>,
    pub velocity: cgmath::Vector3<f32>,
    pub id: u32,
}

#[derive(Default)]
pub struct Particles {
    pub world_size: f32,
    pub current_particles: Vec<Particle>,
    pub previous_particles: Vec<Particle>,
    pub id_count: u32,
    pub attraction_matrix: Vec<f32>,
    pub colors: Vec<cgmath::Vector3<f32>>,
    pub friction_half_time: f32,
    pub force_scale: f32,
    pub particle_effect_radius: f32,
}

impl Particles {
    pub fn update(&mut self, ts: f32) {
        // Apply forces
        {
            assert!(self.world_size >= 2.0 * self.particle_effect_radius);

            let cell_coord = |v: cgmath::Vector3<f32>| -> cgmath::Vector3<isize> {
                cgmath::vec3(
                    (v.x / self.particle_effect_radius) as isize,
                    (v.y / self.particle_effect_radius) as isize,
                    (v.z / self.particle_effect_radius) as isize,
                )
            };

            fn hash(cgmath::Vector3 { x, y, z }: cgmath::Vector3<isize>) -> usize {
                let mut hasher = DefaultHasher::new();
                x.hash(&mut hasher);
                y.hash(&mut hasher);
                z.hash(&mut hasher);
                hasher.finish() as usize
            }

            let hash_table_length = self.current_particles.len();
            let hash_table: Vec<_> = std::iter::repeat_with(|| AtomicUsize::new(0))
                .take(hash_table_length + 1)
                .collect();

            self.current_particles.par_iter().for_each(|sphere| {
                let index = hash(cell_coord(sphere.position)) % hash_table_length;
                hash_table[index].fetch_add(1, Relaxed);
            });

            for i in 1..hash_table.len() {
                hash_table[i].fetch_add(hash_table[i - 1].load(Relaxed), Relaxed);
            }

            let particle_indices: Vec<_> = std::iter::repeat_with(|| AtomicUsize::new(0))
                .take(self.current_particles.len())
                .collect();
            self.current_particles
                .par_iter()
                .enumerate()
                .for_each(|(i, sphere)| {
                    let index = hash(cell_coord(sphere.position)) % hash_table_length;
                    let index = hash_table[index].fetch_sub(1, Relaxed);
                    particle_indices[index - 1].store(i, Relaxed);
                });

            std::mem::swap(&mut self.current_particles, &mut self.previous_particles);
            self.current_particles.clear();
            self.current_particles
                .par_extend(self.previous_particles.par_iter().map(|&(mut particle)| {
                    let mut total_force = cgmath::Vector3::zero();
                    for x_offset in -1..=1 {
                        for y_offset in -1..=1 {
                            for z_offset in -1..=1 {
                                let offset =
                                    cgmath::vec3(x_offset as _, y_offset as _, z_offset as _)
                                        * self.world_size;
                                let cell = cell_coord(particle.position + offset);

                                for x_cell_offset in -1isize..=1 {
                                    for y_cell_offset in -1isize..=1 {
                                        for z_cell_offset in -1isize..=1 {
                                            let cell = cell
                                                + cgmath::vec3(
                                                    x_cell_offset,
                                                    y_cell_offset,
                                                    z_cell_offset,
                                                );

                                            let index = hash(cell) % hash_table_length;
                                            for index in &particle_indices[hash_table[index]
                                                .load(Relaxed)
                                                ..hash_table[index + 1].load(Relaxed)]
                                            {
                                                let other_particle =
                                                    &self.previous_particles[index.load(Relaxed)];

                                                let relative_position = other_particle.position
                                                    - particle.position
                                                    + offset;
                                                let sqr_distance = relative_position.magnitude2();
                                                if sqr_distance > 0.0
                                                    && sqr_distance
                                                        < self.particle_effect_radius
                                                            * self.particle_effect_radius
                                                {
                                                    let distance = sqr_distance.sqrt();
                                                    fn force(r: f32, attraction: f32) -> f32 {
                                                        const BETA: f32 = 0.3;
                                                        if r < BETA {
                                                            r / BETA - 1.0
                                                        } else if BETA < r && r < 1.0 {
                                                            attraction
                                                                * (1.0
                                                                    - (2.0 * r - 1.0 - BETA).abs()
                                                                        / (1.0 - BETA))
                                                        } else {
                                                            0.0
                                                        }
                                                    }
                                                    let f = force(
                                                        distance,
                                                        self.attraction_matrix[(particle.id
                                                            * self.id_count
                                                            + other_particle.id)
                                                            as usize],
                                                    );
                                                    total_force += relative_position / distance * f;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    particle.velocity +=
                        total_force * self.force_scale * self.particle_effect_radius * ts;
                    particle
                }));
        }

        let friction_constant = 0.5f32.powf(self.friction_half_time);
        self.current_particles.par_iter_mut().for_each(|particle| {
            particle.velocity -= particle.velocity * friction_constant * ts;
            particle.position += particle.velocity * ts;
            if particle.position.x > self.world_size * 0.5 {
                particle.position.x -= self.world_size;
            }
            if particle.position.x < -self.world_size * 0.5 {
                particle.position.x += self.world_size;
            }
            if particle.position.y > self.world_size * 0.5 {
                particle.position.y -= self.world_size;
            }
            if particle.position.y < -self.world_size * 0.5 {
                particle.position.y += self.world_size;
            }
            if particle.position.z > self.world_size * 0.5 {
                particle.position.z -= self.world_size;
            }
            if particle.position.z < -self.world_size * 0.5 {
                particle.position.z += self.world_size;
            }
        });
    }
}
