[[block]]
struct Uniforms {
    mouse: vec2<f32>;
    size: vec2<f32>;
    cursor_size: f32;
};

[[group(0), binding(0)]]
var uniforms: Uniforms;

// Vertex shader

struct VertexOutput {
    [[location(0)]] world_pos: vec2<f32>;
    [[builtin(position)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn main([[builtin(vertex_index)]] in_vertex_index: u32) -> VertexOutput {
    var vertices: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
        vec2<f32>(-1., -3.0),
        vec2<f32>(3.0, 1.),
        vec2<f32>(-1., 1.),
    );
    var out: VertexOutput;
    out.position = vec4<f32>(vertices[in_vertex_index], 0.0, 1.0);
    out.world_pos = 0.5 * out.position.xy * uniforms.size;
    return out;
}

// Fragment shader

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] f32 {
    let p = uniforms.mouse - in.world_pos;
    let r = 0.5 * uniforms.cursor_size;
    let q = (p + 1.5 * uniforms.size) % uniforms.size - 0.5 * uniforms.size;
    return length(q) - r;
}