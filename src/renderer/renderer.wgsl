[[block]]
struct Uniforms {
    translate: vec2<f32>;
    view_size: vec2<f32>;
    world_size: vec2<f32>;
    inv_world_size: vec2<f32>;
    pixel_size: vec2<f32>;
    mouse: vec2<f32>;
    cursor_size: f32;
    time: f32;
    exposure: f32;
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

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
var t_sdf: texture_2d<f32>;
[[group(1), binding(1)]]
var s_sdf: sampler;

[[group(2), binding(0)]]
var t_lightmap: texture_2d<f32>;
[[group(2), binding(1)]]
var s_lightmap: sampler;

fn unpackSdf(v: f32) -> f32 {
    return v;
}

fn sceneDist(world_pos: vec2<f32>) -> f32 {
    var uv = world_pos * uniforms.inv_world_size;
    uv.y = -uv.y;
    uv = uv + 0.5;
    return unpackSdf(textureSample(t_sdf, s_sdf, uv).r);
}

fn wrap(p: vec2<f32>) -> vec2<f32> 
{
    let s = ceil(abs(p / uniforms.world_size)) + 0.5;
    return (p + s * uniforms.world_size) % uniforms.world_size - 0.5 * uniforms.world_size;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var col = textureSample(t_lightmap, s_lightmap, in.uv).rgb;

    // reinhard tone mapping
    //col = col / (col + 1.0);

    col = vec3<f32>(1., 1., 1.) - exp(-col * uniforms.exposure);

    let worldPosChange = fwidth(in.world_pos.x);

    //let dist = sceneDist(in.world_pos);
    //let insideField = smoothStep(2. * worldPosChange, 0., abs((-dist + 5.) % 10. - 5.));
    //let outsideField = smoothStep(2. * worldPosChange, 0., abs((dist + 10.) % 20. - 10.));
    //let field = min(0.1, max(insideField, outsideField));
    //col = mix(col, vec3<f32>(1., 1., 1.), field);

    let cursorSize = 0.5 * uniforms.cursor_size;
    let mouseDistance = length(wrap(uniforms.mouse - in.world_pos));

    let CursorThickness = 2.0;
    let CursorCol = vec3<f32>(0., 0., 0.5);

    let cursorAlpha = smoothStep(CursorThickness * worldPosChange, 0., abs(mouseDistance - cursorSize));

    return vec4<f32>(mix(col, CursorCol, cursorAlpha), 1.0);
}