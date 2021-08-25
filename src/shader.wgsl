// Vertex shader

struct VertexOutput {
    [[location(0)]] coord: vec2<f32>;
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
    out.coord = vertices[in_vertex_index];
    out.position = vec4<f32>(out.coord, 0.0, 1.0);
    return out;
}

// Fragment shader

[[block]]
struct Uniforms {
    mousePos: vec2<f32>;
    size: vec2<f32>;
    cursor_size: f32;
};

[[group(0), binding(0)]]
var uniforms: Uniforms;

[[group(1), binding(0)]]
var t_sdf: texture_2d<f32>;
[[group(1), binding(1)]]
var s_sdf: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let cursorSize = 0.5 * uniforms.cursor_size;
    let mouseDistance = length(0.5 * (uniforms.mousePos - in.coord) * uniforms.size);

    let _CursorThickness = 2.0;
    let _CursorCol = vec4<f32>(0., 0., 0.5, 1.);

    let coordChange = 0.5 * fwidth(in.coord.x) * uniforms.size.x;
    let cursorAlpha = smoothStep(cursorSize - _CursorThickness * coordChange, cursorSize - max(_CursorThickness - 1., 0.) * coordChange, mouseDistance) * smoothStep(cursorSize + _CursorThickness * coordChange, cursorSize + max(_CursorThickness - 1., 0.) * coordChange, mouseDistance);

    let normalized = 0.5 * (in.coord + vec2<f32>(1., 1.));
    let dist = textureSample(t_sdf, s_sdf, normalized).r;

    let _InsideColor = vec4<f32>(0.5, 0., 0., 1.);
    let _OutsideColor = vec4<f32>(0., 0.5, 0., 1.);
    let col = mix(_InsideColor, _OutsideColor, step(0., dist));

    let _LineDistance = 100.0;
    let _LineThickness = 2.0;

    let distanceChange = fwidth(dist) * 0.5;
    let majorLineDistance = abs(fract(dist / _LineDistance + 0.5) - 0.5) * _LineDistance;
    let majorLines = smoothStep(_LineThickness - distanceChange, _LineThickness + distanceChange, majorLineDistance);

    let _SubLines = 5.0;
    let _SubLineThickness = 1.0;

    let distanceBetweenSubLines = _LineDistance / _SubLines;
    let subLineDistance = abs(fract(dist / distanceBetweenSubLines + 0.5) - 0.5) * distanceBetweenSubLines;
    let subLines = smoothStep(_SubLineThickness - distanceChange, _SubLineThickness + distanceChange, subLineDistance);

    return mix(col * majorLines * subLines, _CursorCol, cursorAlpha);
}