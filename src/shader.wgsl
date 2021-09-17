[[block]]
struct Uniforms {
    translate: vec2<f32>;
    view_size: vec2<f32>;
    world_size: vec2<f32>;
    inv_world_size: vec2<f32>;
    mouse: vec2<f32>;
    cursor_size: f32;
    time: f32;
};

[[group(0), binding(0)]]
var uniforms: Uniforms;

// Vertex shader

struct VertexOutput {
    [[location(0)]] world_pos: vec2<f32>;
    [[location(1)]] uv: vec2<f32>;
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
    out.world_pos = uniforms.translate + 0.5 * out.position.xy * uniforms.view_size;
    out.uv = 0.5 * out.position.xy * vec2<f32>(1., -1.) + 0.5;
    return out;
}

// Fragment shader

[[group(1), binding(0)]]
var t_result: texture_2d<f32>;
[[group(1), binding(1)]]
var s_result: sampler;

fn wrap(p: vec2<f32>) -> vec2<f32> 
{
    let s = ceil(abs(p / uniforms.world_size)) + 0.5;
    return (p + s * uniforms.world_size) % uniforms.world_size - 0.5 * uniforms.world_size;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let cursorSize = 0.5 * uniforms.cursor_size;
    let mouseDistance = length(wrap(uniforms.mouse - in.world_pos));

    let _CursorThickness = 2.0;
    let _CursorCol = vec4<f32>(0., 0., 0.5, 1.);

    let worldPosChange = fwidth(in.world_pos.x);
    let cursorAlpha = smoothStep(_CursorThickness * worldPosChange, 0., abs(mouseDistance - cursorSize));

    let col = textureSample(t_result, s_result, in.uv);

    return mix(col, _CursorCol, cursorAlpha);
}