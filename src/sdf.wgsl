struct Uniforms {
    world_pos: vec2<f32>,
    world_size: vec2<f32>,
    inv_world_size: vec2<f32>,
    radius: f32,
    smoothness: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Vertex shader

struct VertexOutput {
    @location(0) world_pos: vec2<f32>,
    @builtin(position) position: vec4<f32>,
}

@vertex
fn main_vert(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var vertices: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
        vec2<f32>(-1., -3.0),
        vec2<f32>(3.0, 1.),
        vec2<f32>(-1., 1.),
    );
    var out: VertexOutput;
    out.position = vec4<f32>(vertices[in_vertex_index], 0.0, 1.0);
    out.world_pos = 0.5 * out.position.xy * uniforms.world_size;
    return out;
}

// Fragment shader

fn packSdf(v: f32) -> f32 {
    return v;
}

fn unpackSdf(v: f32) -> f32 {
    return v;
}

@group(1) @binding(0)
var t_sdf: texture_2d<f32>;
@group(1) @binding(1)
var s_sdf: sampler;

fn sceneDist(world_pos: vec2<f32>) -> f32 {
    var uv = world_pos * uniforms.inv_world_size;
    uv.y = -uv.y;
    uv = uv + 0.5;
    return unpackSdf(textureSample(t_sdf, s_sdf, uv).r);
}

fn smoothUnion(d1: f32, d2: f32) -> f32 {
    let h = max(uniforms.smoothness-abs(d1-d2),0.0);
    return min(d1, d2) - h*h*0.25/uniforms.smoothness;
}

fn smoothSubtract(d1: f32, d2: f32) -> f32 {
    return -smoothUnion(-d1, d2);
}

@fragment
fn main_frag(in: VertexOutput) -> @location(0) f32 {
    let p = uniforms.world_pos - in.world_pos;
    let r = uniforms.radius;
    let q = (p + 1.5 * uniforms.world_size) % uniforms.world_size - 0.5 * uniforms.world_size;
    return packSdf(smoothUnion(sceneDist(in.world_pos), length(q) - r));
}

@fragment
fn main_frag_subtract(in: VertexOutput) -> @location(0) f32 {
    let p = uniforms.world_pos - in.world_pos;
    let r = uniforms.radius;
    let q = (p + 1.5 * uniforms.world_size) % uniforms.world_size - 0.5 * uniforms.world_size;
    return packSdf(smoothSubtract(sceneDist(in.world_pos), length(q) - r));
}