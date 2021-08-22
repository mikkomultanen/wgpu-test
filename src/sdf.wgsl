// Vertex shader

struct VertexOutput {
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
    return out;
}

// Fragment shader

[[block]]
struct Uniforms {
    mouse: vec2<f32>;
    size: vec2<f32>;
    cursor_size: f32;
};

[[group(0), binding(0)]]
var uniforms: Uniforms;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] f32 {
    let mouse_screen = 0.5 * (uniforms.mouse + vec2<f32>(1., 1.)) * uniforms.size;
    let d = distance(mouse_screen, in.position.xy);
    let cursor_size = 0.5 * uniforms.cursor_size;
    let delta = fwidth(in.position.x);
    let cursor_a = smoothStep(cursor_size - delta, cursor_size, d);

    return mix(0.0, 1.0, cursor_a);
}