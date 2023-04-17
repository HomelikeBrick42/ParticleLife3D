use ::rand::prelude::*;
use macroquad::prelude::*;
use particle_life_3d::{Particle, Particles};

const CAMERA_SPEED: f32 = 5.0;
const CAMERA_ROTATION_SPEED: f32 = 90.0;
const TIME_STEP: f32 = 1.0 / 60.0;

fn config() -> Conf {
    Conf {
        window_title: "Particle Life 3D".into(),
        icon: None,
        ..Default::default()
    }
}

#[derive(Clone, Copy)]
struct Camera {
    pub position: Vec3,
    pub up: Vec3,
    pub pitch: f32,
    pub yaw: f32,
}

#[derive(Clone, Copy)]
struct Axes {
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

impl Camera {
    pub fn get_axes(&self) -> Axes {
        let forward = vec3(
            self.pitch.to_radians().cos() * (-self.yaw).to_radians().sin(),
            self.pitch.to_radians().sin(),
            self.pitch.to_radians().cos() * (-self.yaw).to_radians().cos(),
        )
        .normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward).normalize();
        Axes { forward, right, up }
    }
}

impl From<Camera> for Camera3D {
    fn from(camera: Camera) -> Self {
        let axes = camera.get_axes();
        Camera3D {
            position: camera.position,
            target: camera.position + axes.forward,
            up: camera.up,
            ..Default::default()
        }
    }
}

#[macroquad::main(config)]
async fn main() {
    let mut particles = Particles {
        world_size: 10.0,
        id_count: 5,
        colors: vec![
            Color::from_vec(vec4(1.0, 0.0, 0.0, 1.0)), // red
            Color::from_vec(vec4(0.0, 1.0, 0.0, 1.0)), // green
            Color::from_vec(vec4(0.0, 0.0, 1.0, 1.0)), // blue
            Color::from_vec(vec4(1.0, 1.0, 0.0, 1.0)), // yellow
            Color::from_vec(vec4(1.0, 0.0, 1.0, 1.0)), // purple
        ],
        attraction_matrix: vec![
            0.5, 1.0, -0.5, 0.0, -1.0, // red
            1.0, 1.0, 1.0, 0.0, -1.0, // green
            0.0, 0.0, 0.5, 1.5, -1.0, // blue
            0.0, 0.0, 0.0, 0.0, -1.0, // yellow
            1.0, 1.0, 1.0, 1.0, 0.5, // purple
        ],
        particle_effect_radius: 2.0,
        friction_half_time: 0.04,
        force_scale: 1.0,
        current_particles: vec![],
        previous_particles: vec![],
    };

    particles.current_particles = {
        let mut rng = thread_rng();
        std::iter::repeat_with(|| Particle {
            position: vec3(
                rng.gen_range(particles.world_size * -0.5..=particles.world_size * 0.5),
                rng.gen_range(particles.world_size * -0.5..=particles.world_size * 0.5),
                rng.gen_range(particles.world_size * -0.5..=particles.world_size * 0.5),
            ),
            velocity: vec3(0.0, 0.0, 0.0),
            id: rng.gen_range(0..5),
        })
        .take(1000)
        .collect()
    };

    let mut camera = Camera {
        position: vec3(0.0, 0.0, -particles.world_size * 1.5),
        up: vec3(0.0, 1.0, 0.0),
        pitch: 0.0,
        yaw: 0.0,
    };

    set_cursor_grab(true);
    show_mouse(false);

    let mut fixed_time = 0.0;
    loop {
        let ts = get_frame_time();
        fixed_time += ts;

        let start_update = std::time::Instant::now();
        if fixed_time >= TIME_STEP {
            particles.update(ts);
            fixed_time -= TIME_STEP;
        }
        let update_elapsed = start_update.elapsed();

        {
            let axes = camera.get_axes();
            if is_key_down(KeyCode::W) {
                camera.position += axes.forward * CAMERA_SPEED * ts;
            }
            if is_key_down(KeyCode::S) {
                camera.position -= axes.forward * CAMERA_SPEED * ts;
            }
            if is_key_down(KeyCode::A) {
                camera.position -= axes.right * CAMERA_SPEED * ts;
            }
            if is_key_down(KeyCode::D) {
                camera.position += axes.right * CAMERA_SPEED * ts;
            }
            if is_key_down(KeyCode::Q) {
                camera.position -= axes.up * CAMERA_SPEED * ts;
            }
            if is_key_down(KeyCode::E) {
                camera.position += axes.up * CAMERA_SPEED * ts;
            }

            if is_key_down(KeyCode::Up) {
                camera.pitch += CAMERA_ROTATION_SPEED * ts;
            }
            if is_key_down(KeyCode::Down) {
                camera.pitch -= CAMERA_ROTATION_SPEED * ts;
            }
            if is_key_down(KeyCode::Left) {
                camera.yaw -= CAMERA_ROTATION_SPEED * ts;
            }
            if is_key_down(KeyCode::Right) {
                camera.yaw += CAMERA_ROTATION_SPEED * ts;
            }

            camera.pitch = camera.pitch.clamp(-89.9999, 89.9999);
        }

        clear_background(Color::from_vec(vec4(0.1, 0.1, 0.1, 1.0)));
        set_camera(&Camera3D::from(camera));

        draw_cube_wires(
            vec3(0.0, 0.0, 0.0),
            vec3(
                particles.world_size,
                particles.world_size,
                particles.world_size,
            ),
            Color::from_vec(vec4(1.0, 1.0, 1.0, 1.0)),
        );

        for particle in &particles.current_particles {
            draw_sphere(particle.position, 0.05, None, particles.colors[particle.id]);
        }

        set_default_camera();
        draw_text(&format!("FPS: {:.3}", 1.0 / ts), 5.0, 16.0, 16.0, WHITE);
        draw_text(
            &format!("Frame Time: {:.3}ms", ts * 1000.0),
            5.0,
            32.0,
            16.0,
            WHITE,
        );
        draw_text(
            &format!(
                "Update Time: {:.3}ms",
                update_elapsed.as_secs_f64() * 1000.0
            ),
            5.0,
            48.0,
            16.0,
            WHITE,
        );

        next_frame().await;
    }
}
