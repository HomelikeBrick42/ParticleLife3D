struct VertexIn {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) particle_index: u32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) particle_index: u32,
};

struct Camera {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
};

@group(0)
@binding(0)
var<uniform> camera: Camera;

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    id: u32,
};

struct Particles {
    world_size: f32,
    length: u32,
    particles: array<Particle>,
};

@group(1)
@binding(0)
var<storage, read> particles: Particles;

struct Colors {
    length: u32,
    colors: array<vec3<f32>>,
};

@group(1)
@binding(1)
var<storage, read> colors: Colors;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.particle_index = in.particle_index;

    out.uv = vec2(f32((in.vertex_index >> 0u) & 1u), f32((in.vertex_index >> 1u) & 1u));

    out.position = camera.view_matrix * vec4(particles.particles[in.particle_index].position, 1.0);
    out.position += vec4(
        (out.uv - 0.5) * 0.1,
        0.0,
        0.0,
    );
    out.position = camera.projection_matrix * out.position;

    out.world_position = out.position.xyz / out.position.w;

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if length(in.uv * 2.0 - 1.0) > 1.0 {
        discard;
    }
    return vec4(colors.colors[particles.particles[in.particle_index].id], 1.0);
}
