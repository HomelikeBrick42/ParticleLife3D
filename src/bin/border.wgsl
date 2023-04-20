struct VertexIn {
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
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

// TODO: change this to a `const` when naga is fixed
var<private> vertices: array<vec3<f32>, 24> = array<vec3<f32>, 24>(
    vec3<f32>(-1.0, 1.0, 1.0),
    vec3<f32>(-1.0, -1.0, 1.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(1.0, -1.0, 1.0),
    vec3<f32>(-1.0, 1.0, 1.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(-1.0, -1.0, 1.0),
    vec3<f32>(1.0, -1.0, 1.0),
    vec3<f32>(-1.0, 1.0, -1.0),
    vec3<f32>(-1.0, -1.0, -1.0),
    vec3<f32>(1.0, 1.0, -1.0),
    vec3<f32>(1.0, -1.0, -1.0),
    vec3<f32>(-1.0, 1.0, -1.0),
    vec3<f32>(1.0, 1.0, -1.0),
    vec3<f32>(-1.0, -1.0, -1.0),
    vec3<f32>(1.0, -1.0, -1.0),
    vec3<f32>(1.0, 1.0, -1.0),
    vec3<f32>(1.0, 1.0, 1.0),
    vec3<f32>(-1.0, 1.0, -1.0),
    vec3<f32>(-1.0, 1.0, 1.0),
    vec3<f32>(1.0, -1.0, -1.0),
    vec3<f32>(1.0, -1.0, 1.0),
    vec3<f32>(-1.0, -1.0, -1.0),
    vec3<f32>(-1.0, -1.0, 1.0),
);

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    out.position = camera.projection_matrix * (camera.view_matrix * vec4(vertices[in.vertex_index] * particles.world_size * 0.5, 1.0));
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4(1.0);
}
