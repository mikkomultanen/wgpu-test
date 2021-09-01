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

fn hardShadow(p: vec2<f32>, lightDir: vec2<f32>, lightDistance: f32, radius: f32) -> f32 {
    if (lightDistance < radius) {
        return 1.;
    }
    var d: f32 = 0.0;
    for(var i: i32 = 0; i < 32; i = i + 1) {
        let h = sceneDist(p + d * lightDir);
        if( h < .001) {
            return 0.;
        }            
        d = d + h;
        if(d > lightDistance - radius) {
            return 1.;
        }
    }
    return 0.;
}

fn softShadow(p: vec2<f32>, lightDir: vec2<f32>, lightDistance: f32, radius: f32) -> f32 {
    if (lightDistance <= radius) {
        return 1.;
    }
    var r: f32 = 1.0;
    var d: f32 = 0.02;
    var ph: f32 = 1.0e20;
    let k = radius / lightDistance;
    //let k = radius * inverseSqrt(lightDistance * lightDistance - radius * radius);
    for(var i: i32 = 0; i < 64; i = i + 1) {
        let extra = d * k;
        let h = sceneDist(p + d * lightDir) + extra;
        if( h < .001) {
            return 0.;
        }
        if(d + h - extra > lightDistance - radius) {
            return r;
        }
        let y = h*h/(2.0*ph);
        let t = sqrt(h*h-y*y);
        r = min(r, t/max(0.0,(d + h - y)*k));
        ph = h;
        d = d + .5 * h;
    }
    return 0.;
}

fn drawLight(p: vec2<f32>, pos: vec2<f32>, color: vec4<f32>, dist: f32, range: f32, radius: f32, pChange: f32) -> vec4<f32>
{
    let d = pos - p;
	// distance to light
	let ld = length(d);
	
	// out of range
	if (ld > range) {
        return vec4<f32>(0., 0., 0., 1.);
    }
	
	// shadow and falloff
	let shad = softShadow(p, d / max(radius, ld), ld, radius);
	var fall = (range - ld + radius) / range;
	fall = fall * fall;
	let source = 1.0 - clamp((ld - radius) / pChange + 0.5, 0.0, 1.0);
    return mix(shad * fall, 4., source) * color;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let cursorSize = 0.5 * uniforms.cursor_size;
    let mouseDistance = length(uniforms.mouse - in.world_pos);

    let _CursorThickness = 2.0;
    let _CursorCol = vec4<f32>(0., 0., 0.5, 1.);

    let worldPosChange = fwidth(in.world_pos.x);
    let cursorAlpha = smoothStep(_CursorThickness * worldPosChange, 0., abs(mouseDistance - cursorSize));

    let dist = sceneDist(in.world_pos);

    var _InsideColor = vec4<f32>(1.0, 0.4, 0.0, 1.0);
    let insideField = smoothStep(2. * worldPosChange, 0., abs((-dist + 5.) % 10. - 5.));
    _InsideColor = mix(_InsideColor, vec4<f32>(.5, .2, 0.0, 1.0), insideField);

    var _OutsideColor = vec4<f32>(0.5, 0.5, 0.5, 1.0);
    let p = in.world_pos + 0.5 * uniforms.size;
    let grid = smoothStep(2. * worldPosChange, 0., min(abs(p.x % 10. - 5.), abs(p.y % 10. - 5.)));
    _OutsideColor = mix(_OutsideColor, vec4<f32>(.4, .4, .4, 1.), grid);
    let field = smoothStep(2. * worldPosChange, 0., abs((dist + 10.) % 20. - 10.));
    _OutsideColor = mix(_OutsideColor, vec4<f32>(.3, .3, .3, 1.), field);
    var light = drawLight(in.world_pos, uniforms.mouse, vec4<f32>(0.75, 1.0, 0.5, 1.0), dist, 0.5 * uniforms.size.x, 10.0, worldPosChange);
    light.a = 1.0;
    _OutsideColor = _OutsideColor * light;

    let col = mix(_InsideColor, _OutsideColor, clamp(dist / worldPosChange + 0.5, 0.0, 1.0));

    return mix(col, _CursorCol, cursorAlpha);
}