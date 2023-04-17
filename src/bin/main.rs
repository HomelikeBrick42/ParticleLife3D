use cgmath::prelude::*;
use eframe::egui;
use particle_life_3d::{Particle, Particles};
use rand::prelude::*;

const CAMERA_SPEED: f32 = 5.0;
const CAMERA_ROTATION_SPEED: f32 = 90.0;
const TIME_STEP: f32 = 1.0 / 60.0;

#[derive(Clone, Copy)]
struct Camera {
    pub position: cgmath::Vector3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub pitch: f32,
    pub yaw: f32,
}

#[derive(Clone, Copy)]
struct Axes {
    pub forward: cgmath::Vector3<f32>,
    pub right: cgmath::Vector3<f32>,
    pub up: cgmath::Vector3<f32>,
}

impl Camera {
    pub fn get_axes(&self) -> Axes {
        let forward = cgmath::vec3(
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

struct App {
    particles: Particles,
    camera: Camera,
    last_time: std::time::Instant,
    fixed_time: std::time::Duration,
}

impl App {
    fn new(_cc: &eframe::CreationContext) -> Self {
        let mut particles = Particles {
            world_size: 10.0,
            id_count: 5,
            colors: vec![
                cgmath::vec3(1.0, 0.0, 0.0), // red
                cgmath::vec3(0.0, 1.0, 0.0), // green
                cgmath::vec3(0.0, 0.0, 1.0), // blue
                cgmath::vec3(1.0, 1.0, 0.0), // yellow
                cgmath::vec3(1.0, 0.0, 1.0), // purple
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
                position: cgmath::vec3(
                    rng.gen_range(particles.world_size * -0.5..=particles.world_size * 0.5),
                    rng.gen_range(particles.world_size * -0.5..=particles.world_size * 0.5),
                    rng.gen_range(particles.world_size * -0.5..=particles.world_size * 0.5),
                ),
                velocity: cgmath::vec3(0.0, 0.0, 0.0),
                id: rng.gen_range(0..5),
            })
            .take(1000)
            .collect()
        };

        let camera = Camera {
            position: cgmath::vec3(0.0, 0.0, -particles.world_size * 1.5),
            up: cgmath::vec3(0.0, 1.0, 0.0),
            pitch: 0.0,
            yaw: 0.0,
        };

        Self {
            particles,
            camera,
            last_time: std::time::Instant::now(),
            fixed_time: std::time::Duration::ZERO,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let time = std::time::Instant::now();
        let ts = time.duration_since(self.last_time);
        self.last_time = time;

        self.fixed_time += ts;
        let start_update = std::time::Instant::now();
        if self.fixed_time.as_secs_f32() >= TIME_STEP {
            self.particles.update(TIME_STEP);
            self.fixed_time -= std::time::Duration::from_secs_f32(TIME_STEP);
        }
        let update_elapsed = start_update.elapsed();

        let ts = ts.as_secs_f32();

        egui::SidePanel::left("Left Panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(format!("FPS: {:.3}", 1.0 / ts));
                ui.label(format!("Frame Time: {:.3}ms", ts * 1000.0));
                ui.label(format!(
                    "Update Time: {:.3}ms",
                    update_elapsed.as_secs_f64() * 1000.0
                ));
                ui.allocate_space(ui.available_size());
            });
        });

        let response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag());
                ui.painter().add(egui::PaintCallback {
                    rect,
                    callback: std::sync::Arc::new(
                        eframe::egui_wgpu::CallbackFn::new()
                            .prepare(
                                move |_device, _queue, _encoder, _paint_callback_resources| {
                                    Vec::new()
                                },
                            )
                            .paint(move |_info, _render_pass, _paint_callback_resources| {}),
                    ),
                });

                response
            })
            .inner;

        if response.has_focus() {
            ctx.input(|i| {
                let axes = self.camera.get_axes();

                if i.key_pressed(egui::Key::W) {
                    self.camera.position += axes.forward * CAMERA_SPEED * ts;
                }
                if i.key_pressed(egui::Key::S) {
                    self.camera.position -= axes.forward * CAMERA_SPEED * ts;
                }
                if i.key_pressed(egui::Key::A) {
                    self.camera.position -= axes.right * CAMERA_SPEED * ts;
                }
                if i.key_pressed(egui::Key::D) {
                    self.camera.position += axes.right * CAMERA_SPEED * ts;
                }
                if i.key_pressed(egui::Key::Q) {
                    self.camera.position -= axes.up * CAMERA_SPEED * ts;
                }
                if i.key_pressed(egui::Key::E) {
                    self.camera.position += axes.up * CAMERA_SPEED * ts;
                }

                if i.key_pressed(egui::Key::ArrowUp) {
                    self.camera.pitch += CAMERA_ROTATION_SPEED * ts;
                }
                if i.key_pressed(egui::Key::ArrowDown) {
                    self.camera.pitch -= CAMERA_ROTATION_SPEED * ts;
                }
                if i.key_pressed(egui::Key::ArrowLeft) {
                    self.camera.yaw -= CAMERA_ROTATION_SPEED * ts;
                }
                if i.key_pressed(egui::Key::ArrowRight) {
                    self.camera.yaw += CAMERA_ROTATION_SPEED * ts;
                }

                self.camera.pitch = self.camera.pitch.clamp(-89.9999, 89.9999);
            });
        }

        ctx.request_repaint();
    }
}

fn main() {
    eframe::run_native(
        "Particle Physics 3D",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            vsync: false,
            ..Default::default()
        },
        Box::new(|cc| Box::new(App::new(cc))),
    )
    .unwrap();
}
