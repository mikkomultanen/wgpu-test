[[block]]
struct Uniforms {
    mouse: vec2<f32>;
    size: vec2<f32>;
    inv_size: vec2<f32>;
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

[[group(1), binding(0)]]
var t_sdf: texture_2d<f32>;
[[group(1), binding(1)]]
var s_sdf: sampler;

fn sceneDist(world_pos: vec2<f32>) -> f32 {
    var uv = world_pos * uniforms.inv_size;
    uv.y = -uv.y;
    uv = uv + 0.5;
    return textureSample(t_sdf, s_sdf, uv).r;
}

fn drawLight(p: vec2<f32>, pos: vec2<f32>, color: vec4<f32>, dist: f32, range: f32, radius: f32, pChange: f32) -> vec4<f32>
{
	// distance to light
	let ld = length(p - pos);
	
	// out of range
	if (ld > range) {
        return vec4<f32>(0., 0., 0., 1.);
    }
	
	// shadow and falloff
	//float shad = shadow(p, pos, radius);
    let shad = 1.0;
	var fall = (range - ld) / range;
	fall = fall * fall;
	let source = 1.0 - clamp((ld - radius) / pChange + 0.5, 0.0, 1.0);
	return (shad * fall + source) * color;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let cursorSize = 0.5 * uniforms.cursor_size;
    let mouseDistance = length(uniforms.mouse - in.world_pos);

    let _CursorThickness = 2.0;
    let _CursorCol = vec4<f32>(0., 0., 0.5, 1.);

    let worldPosChange = 0.5 * fwidth(in.world_pos.x);
    let cursorAlpha = smoothStep(cursorSize - _CursorThickness * worldPosChange, cursorSize - max(_CursorThickness - 1., 0.) * worldPosChange, mouseDistance) * smoothStep(cursorSize + _CursorThickness * worldPosChange, cursorSize + max(_CursorThickness - 1., 0.) * worldPosChange, mouseDistance);

    let dist = sceneDist(in.world_pos);

    let _InsideColor = vec4<f32>(1.0, 0.4, 0.0, 1.0);
    let p = in.world_pos + 0.5 * uniforms.size;
    var _OutsideColor = vec4<f32>(0.5, 0.5, 0.5, 1.0) * clamp(min(p.y % 10.0, p.x % 10.0), 0.9, 1.0);// * clamp(dist % 20.0, 0.8, 1.0);
    var light = drawLight(in.world_pos, uniforms.mouse, vec4<f32>(0.75, 1.0, 0.5, 1.0), dist, 0.5 * uniforms.size.x, 5.0, worldPosChange);
    light.a = 1.0;
    _OutsideColor = _OutsideColor * light;
    let col = mix(_InsideColor, _OutsideColor, clamp(dist / worldPosChange + 0.5, 0.0, 1.0));

    return mix(col, _CursorCol, cursorAlpha);
}