use cgmath::prelude::*;
use eframe::egui_wgpu::wgpu;
use eframe::wgpu::include_wgsl;
use eframe::{egui, wgpu::util::DeviceExt};
use encase::{ArrayLength, ShaderSize, ShaderType, StorageBuffer, UniformBuffer};
use particle_life_3d::{Particle, Particles};
use rand::prelude::*;

const CAMERA_SPEED: f32 = 5.0;
const CAMERA_ROTATION_SPEED: f32 = 90.0;

struct Camera {
    pub position: cgmath::Vector3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub pitch: f32,
    pub yaw: f32,
}

struct Axes {
    pub forward: cgmath::Vector3<f32>,
    pub right: cgmath::Vector3<f32>,
    pub up: cgmath::Vector3<f32>,
}

impl Camera {
    pub fn get_axes(&self) -> Axes {
        let forward = cgmath::vec3(
            self.pitch.to_radians().cos() * self.yaw.to_radians().sin(),
            self.pitch.to_radians().sin(),
            -self.pitch.to_radians().cos() * self.yaw.to_radians().cos(),
        )
        .normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward).normalize();
        Axes { forward, right, up }
    }
}

#[derive(ShaderType)]
struct GpuParticles<'a> {
    pub world_size: f32,
    pub length: ArrayLength,
    #[size(runtime)]
    pub particles: &'a [Particle],
}

#[derive(ShaderType)]
struct GpuColors<'a> {
    pub length: ArrayLength,
    #[size(runtime)]
    pub particles: &'a [cgmath::Vector3<f32>],
}

#[derive(ShaderType)]
struct GpuCamera {
    pub view_matrix: cgmath::Matrix4<f32>,
    pub projection_matrix: cgmath::Matrix4<f32>,
}

struct App {
    particles: Particles,
    camera: Camera,
    last_time: std::time::Instant,
    fixed_time: std::time::Duration,
    ticks_per_second: f32,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
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
            friction: 0.97,
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
            position: cgmath::vec3(1.0, 0.0, particles.world_size * 1.6),
            up: cgmath::vec3(0.0, 1.0, 0.0),
            pitch: 0.0,
            yaw: 0.0,
        };

        let app = Self {
            particles,
            camera,
            last_time: std::time::Instant::now(),
            fixed_time: std::time::Duration::ZERO,
            ticks_per_second: 60.0,
        };

        let render_state = cc.wgpu_render_state.as_ref().unwrap();
        let renderer = Renderer::new(render_state);
        render_state
            .renderer
            .write()
            .paint_callback_resources
            .insert(renderer);

        app
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let time = std::time::Instant::now();
        let ts = time.duration_since(self.last_time);
        self.last_time = time;

        self.fixed_time += ts;
        let start_update = std::time::Instant::now();
        if self.fixed_time.as_secs_f32() >= 1.0 / self.ticks_per_second {
            let ts = 1.0 / self.ticks_per_second;
            self.particles.update(ts);
            self.fixed_time -= std::time::Duration::from_secs_f32(1.0 / self.ticks_per_second);
        }
        let update_elapsed = start_update.elapsed();

        let ts = ts.as_secs_f32();

        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                let axes = self.camera.get_axes();

                if i.key_down(egui::Key::W) {
                    self.camera.position += axes.forward * CAMERA_SPEED * ts;
                }
                if i.key_down(egui::Key::S) {
                    self.camera.position -= axes.forward * CAMERA_SPEED * ts;
                }
                if i.key_down(egui::Key::A) {
                    self.camera.position -= axes.right * CAMERA_SPEED * ts;
                }
                if i.key_down(egui::Key::D) {
                    self.camera.position += axes.right * CAMERA_SPEED * ts;
                }
                if i.key_down(egui::Key::Q) {
                    self.camera.position -= axes.up * CAMERA_SPEED * ts;
                }
                if i.key_down(egui::Key::E) {
                    self.camera.position += axes.up * CAMERA_SPEED * ts;
                }

                if i.key_down(egui::Key::ArrowUp) {
                    self.camera.pitch += CAMERA_ROTATION_SPEED * ts;
                }
                if i.key_down(egui::Key::ArrowDown) {
                    self.camera.pitch -= CAMERA_ROTATION_SPEED * ts;
                }
                if i.key_down(egui::Key::ArrowLeft) {
                    self.camera.yaw -= CAMERA_ROTATION_SPEED * ts;
                }
                if i.key_down(egui::Key::ArrowRight) {
                    self.camera.yaw += CAMERA_ROTATION_SPEED * ts;
                }

                self.camera.pitch = self.camera.pitch.clamp(-89.9999, 89.9999);
            });
        }

        egui::SidePanel::left("Left Panel").show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(format!("FPS: {:.3}", 1.0 / ts));
                ui.label(format!("Frame Time: {:.3}ms", ts * 1000.0));
                ui.label(format!(
                    "Update Time: {:.3}ms",
                    update_elapsed.as_secs_f64() * 1000.0
                ));
                ui.horizontal(|ui| {
                    ui.label("World Size: ");
                    ui.add(egui::DragValue::new(&mut self.particles.world_size).speed(0.1));
                    self.particles.world_size = self
                        .particles
                        .world_size
                        .max(self.particles.particle_effect_radius * 2.0);
                });
                ui.horizontal(|ui| {
                    ui.label("Ticks Per Second: ");
                    ui.add(egui::Slider::new(&mut self.ticks_per_second, 1.0..=1000.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Friction: ");
                    ui.add(
                        egui::Slider::new(&mut self.particles.friction, 0.0..=1.0)
                            .drag_value_speed(0.01),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Force Scale: ");
                    ui.add(egui::Slider::new(
                        &mut self.particles.force_scale,
                        0.0..=10.0,
                    ));
                });
                ui.allocate_space(ui.available_size());
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(ctx.style().visuals.panel_fill))
            .show(ctx, |ui| {
                let (rect, _response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

                let mut camera_uniform =
                    UniformBuffer::new([0; <GpuCamera as ShaderSize>::SHADER_SIZE.get() as _]);
                camera_uniform
                    .write(&{
                        let axes = self.camera.get_axes();
                        GpuCamera {
                            view_matrix: cgmath::Matrix4::look_to_rh(
                                cgmath::point3(
                                    self.camera.position.x,
                                    self.camera.position.y,
                                    self.camera.position.z,
                                ),
                                axes.forward,
                                axes.up,
                            ),
                            projection_matrix: cgmath::perspective(
                                cgmath::Rad::from(cgmath::Deg(90.0)),
                                rect.width() / rect.height(),
                                0.001,
                                1000.0,
                            ),
                        }
                    })
                    .unwrap();
                let camera = camera_uniform.into_inner();

                let mut particles_storage = StorageBuffer::new(vec![]);
                particles_storage
                    .write(&GpuParticles {
                        world_size: self.particles.world_size,
                        length: ArrayLength,
                        particles: &self.particles.current_particles,
                    })
                    .unwrap();
                let particles = particles_storage.into_inner();

                let mut colors_storage = StorageBuffer::new(vec![]);
                colors_storage
                    .write(&GpuColors {
                        length: ArrayLength,
                        particles: &self.particles.colors,
                    })
                    .unwrap();
                let colors = colors_storage.into_inner();

                let sphere_count = self.particles.current_particles.len();

                ui.painter().add(egui::PaintCallback {
                    rect,
                    callback: std::sync::Arc::new(
                        eframe::egui_wgpu::CallbackFn::new()
                            .prepare(move |device, queue, encoder, paint_callback_resources| {
                                let renderer: &mut Renderer =
                                    paint_callback_resources.get_mut().unwrap();
                                renderer
                                    .prepare(&camera, &particles, &colors, device, queue, encoder)
                            })
                            .paint(move |_info, render_pass, paint_callback_resources| {
                                let renderer: &Renderer = paint_callback_resources.get().unwrap();
                                renderer.paint(sphere_count as _, render_pass);
                            }),
                    ),
                });
            });

        ctx.request_repaint();
    }
}

struct Renderer {
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    particles_storage_buffer: wgpu::Buffer,
    particles_storage_buffer_size: usize,
    colors_storage_buffer: wgpu::Buffer,
    colors_storage_buffer_size: usize,
    particles_bind_group_layout: wgpu::BindGroupLayout,
    particles_bind_group: wgpu::BindGroup,
    particles_render_pipeline: wgpu::RenderPipeline,
    border_render_pipeline: wgpu::RenderPipeline,
}

impl Renderer {
    fn new(render_state: &eframe::egui_wgpu::RenderState) -> Self {
        let particles_shader = render_state
            .device
            .create_shader_module(include_wgsl!("./particles.wgsl"));

        let border_shader = render_state
            .device
            .create_shader_module(include_wgsl!("./border.wgsl"));

        let camera_bind_group_layout =
            render_state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Camera Bind Group Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(<GpuCamera as ShaderSize>::SHADER_SIZE),
                        },
                        count: None,
                    }],
                });

        let camera_uniform_buffer =
            render_state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera Uniform Buffer"),
                    contents: &[0; <GpuCamera as ShaderSize>::SHADER_SIZE.get() as _],
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                });

        let camera_bind_group = render_state
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Camera Bind Group"),
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                }],
            });

        let particles_bind_group_layout =
            render_state
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Particles Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: Some(<GpuParticles as ShaderType>::min_size()),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: Some(<GpuColors as ShaderType>::min_size()),
                            },
                            count: None,
                        },
                    ],
                });

        const PARTICLES_STORAGE_BUFFER_SIZE: usize =
            <GpuParticles as ShaderType>::METADATA.min_size().get() as _;
        let particles_storage_buffer =
            render_state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particles Storage Buffer"),
                    contents: &[0; PARTICLES_STORAGE_BUFFER_SIZE],
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                });

        const COLORS_STORAGE_BUFFER_SIZE: usize =
            <GpuColors as ShaderType>::METADATA.min_size().get() as _;
        let colors_storage_buffer =
            render_state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Particles Storage Buffer"),
                    contents: &[0; COLORS_STORAGE_BUFFER_SIZE],
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                });

        let particles_bind_group =
            render_state
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Particles Bind Group"),
                    layout: &particles_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: particles_storage_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: colors_storage_buffer.as_entire_binding(),
                        },
                    ],
                });

        let particles_pipeline_layout =
            render_state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Particles Pipeline Layout"),
                    bind_group_layouts: &[&camera_bind_group_layout, &particles_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let particles_render_pipeline =
            render_state
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Particles Render Pipeline"),
                    layout: Some(&particles_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &particles_shader,
                        entry_point: "vs_main",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &particles_shader,
                        entry_point: "fs_main",
                        targets: &[Some(render_state.target_format.into())],
                    }),
                    primitive: wgpu::PrimitiveState {
                        polygon_mode: wgpu::PolygonMode::Fill,
                        topology: wgpu::PrimitiveTopology::TriangleStrip,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        ..Default::default()
                    },
                    multiview: None,
                });

        let border_pipeline_layout =
            render_state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Border Pipeline Layout"),
                    bind_group_layouts: &[&camera_bind_group_layout, &particles_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let border_render_pipeline =
            render_state
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Border Render Pipeline"),
                    layout: Some(&border_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &border_shader,
                        entry_point: "vs_main",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &border_shader,
                        entry_point: "fs_main",
                        targets: &[Some(render_state.target_format.into())],
                    }),
                    primitive: wgpu::PrimitiveState {
                        polygon_mode: wgpu::PolygonMode::Line,
                        topology: wgpu::PrimitiveTopology::LineList,
                        ..Default::default()
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: wgpu::MultisampleState {
                        ..Default::default()
                    },
                    multiview: None,
                });

        Self {
            camera_uniform_buffer,
            camera_bind_group,
            particles_storage_buffer,
            particles_storage_buffer_size: PARTICLES_STORAGE_BUFFER_SIZE,
            colors_storage_buffer,
            colors_storage_buffer_size: COLORS_STORAGE_BUFFER_SIZE,
            particles_bind_group_layout,
            particles_bind_group,
            particles_render_pipeline,
            border_render_pipeline,
        }
    }

    fn prepare(
        &mut self,
        camera: &[u8],
        particles: &[u8],
        colors: &[u8],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &wgpu::CommandEncoder,
    ) -> Vec<wgpu::CommandBuffer> {
        // Update camera
        queue.write_buffer(&self.camera_uniform_buffer, 0, camera);

        // Update particles and colors
        {
            let mut particles_bind_group_invalidated = false;
            if self.particles_storage_buffer_size >= particles.len() {
                queue.write_buffer(&self.particles_storage_buffer, 0, particles);
            } else {
                particles_bind_group_invalidated = true;
                self.particles_storage_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Particles Storage Buffer"),
                        contents: particles,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                    });
                self.particles_storage_buffer_size = particles.len();
            }
            if self.colors_storage_buffer_size >= particles.len() {
                queue.write_buffer(&self.colors_storage_buffer, 0, colors);
            } else {
                particles_bind_group_invalidated = true;
                self.colors_storage_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Particles Storage Buffer"),
                        contents: colors,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                    });
                self.colors_storage_buffer_size = colors.len();
            }
            if particles_bind_group_invalidated {
                self.particles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Particles Bind Group"),
                    layout: &self.particles_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.particles_storage_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.colors_storage_buffer.as_entire_binding(),
                        },
                    ],
                });
            }
        }

        vec![]
    }

    fn paint<'a>(&'a self, sphere_count: u32, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.particles_render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.particles_bind_group, &[]);
        render_pass.draw(0..4, 0..sphere_count);

        render_pass.set_pipeline(&self.border_render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.particles_bind_group, &[]);
        render_pass.draw(0..24, 0..1);
    }
}

fn main() {
    eframe::run_native(
        "Particle Physics 3D",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
                present_mode: wgpu::PresentMode::AutoNoVsync,
                depth_format: Some(wgpu::TextureFormat::Depth32Float),
                device_descriptor: wgpu::DeviceDescriptor {
                    features: wgpu::Features::POLYGON_MODE_LINE,
                    ..Default::default()
                },
                ..Default::default()
            },
            vsync: false,
            depth_buffer: 32,
            ..Default::default()
        },
        Box::new(|cc| Box::new(App::new(cc))),
    )
    .unwrap();
}
