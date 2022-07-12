struct VertexOutput {
    @location(0) uv: vec2<f32>,
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
    out.uv = 0.5 * out.position.xy * vec2<f32>(1., -1.) + 0.5;
    return out;
}

@group(0) @binding(0)
var t_colorTexture: texture_2d<f32>;
@group(0) @binding(1)
var s_colorTexture: sampler;

@fragment
fn main_frag(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_colorTexture, s_colorTexture, in.uv);
}