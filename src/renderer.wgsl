[[block]]
struct Uniforms {
    translate: vec2<f32>;
    view_size: vec2<f32>;
    mouse: vec2<f32>;
    world_size: vec2<f32>;
    inv_world_size: vec2<f32>;
    time: f32;
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
    out.world_pos = uniforms.translate + 0.5 * out.position.xy * uniforms.view_size;
    return out;
}

// Fragment shader

[[group(1), binding(0)]]
var t_sdf: texture_2d<f32>;
[[group(1), binding(1)]]
var s_sdf: sampler;

fn sceneDist(world_pos: vec2<f32>) -> f32 {
    var uv = world_pos * uniforms.inv_world_size;
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

fn wrap(p: vec2<f32>) -> vec2<f32> 
{
    return (p + 1.5 * uniforms.world_size) % uniforms.world_size - 0.5 * uniforms.world_size;
}

fn drawLight(p: vec2<f32>, pos: vec2<f32>, color: vec4<f32>, dist: f32, range: f32, radius: f32, pChange: f32) -> vec4<f32>
{
    let d = wrap(pos - p);
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
    let worldPosChange = fwidth(in.world_pos.x);

    let dist = sceneDist(in.world_pos);

    var _InsideColor = vec4<f32>(1.0, 0.4, 0.0, 1.0);
    let insideField = smoothStep(2. * worldPosChange, 0., abs((-dist + 5.) % 10. - 5.));
    _InsideColor = mix(_InsideColor, vec4<f32>(.5, .2, 0.0, 1.0), insideField);

    var _OutsideColor = vec4<f32>(0.5, 0.5, 0.5, 1.0);
    let field = smoothStep(2. * worldPosChange, 0., abs((dist + 10.) % 20. - 10.));
    _OutsideColor = mix(_OutsideColor, vec4<f32>(.3, .3, .3, 1.), field);
    var light = drawLight(in.world_pos, uniforms.mouse, vec4<f32>(0.75, 1.0, 0.5, 1.0), dist, 500.0, 10.0, worldPosChange);
    light.a = 1.0;
    _OutsideColor = _OutsideColor * light;

    let col = mix(_InsideColor, _OutsideColor, clamp(dist / worldPosChange + 0.5, 0.0, 1.0));

    return col;
}

fn lightDist(p: vec2<f32>) -> f32 {
    let q = wrap(uniforms.mouse - p);
    return length(q) - 10.;
}

fn trace(p: vec2<f32>, dir: vec2<f32>, worldPosChange: f32) -> vec4<f32>
{
    var dl = 0.02;
    var d: vec2<f32> = p + dl * dir;
    let range = 500.0;
    for(var i: i32 = 0; i < 16; i = i + 1) {
        let h = sceneDist(d);
        let l = lightDist(d) + 1.;
        if( h < worldPosChange) {
            return vec4<f32>(0., 0., 0., 1.);
        }
        if( l - 1. < worldPosChange) {
            let fall = (range - dl - l + 10.0) / range;
            return vec4<f32>(0.75, 1.0, 0.5, 1.0) * fall;
        }
        let m = min(h, l);
        dl = dl + m;
        if(dl > range) {
            return vec4<f32>(0., 0., 0., 1.);
        }
        d = d +  m * dir;
    }
    return vec4<f32>(0., 0., 0., 1.);
}

fn random(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898,78.233)))* 43758.5453123);
}

let SAMPLES: u32 = 16u;

[[stage(fragment)]]
fn main_gi(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let worldPosChange = fwidth(in.world_pos.x);

    let dist = sceneDist(in.world_pos);

    var _InsideColor = vec4<f32>(1.0, 0.4, 0.0, 1.0);
    let insideField = smoothStep(2. * worldPosChange, 0., abs((-dist + 5.) % 10. - 5.));
    _InsideColor = mix(_InsideColor, vec4<f32>(.5, .2, 0.0, 1.0), insideField);

    var _OutsideColor = vec4<f32>(0.5, 0.5, 0.5, 1.0);
    let field = smoothStep(2. * worldPosChange, 0., abs((dist + 10.) % 20. - 10.));
    _OutsideColor = mix(_OutsideColor, vec4<f32>(.3, .3, .3, 1.), field);
    var light = vec4<f32>(0., 0., 0., 1.);
    for (var i = 0u; i < SAMPLES; i = i + 1u) {
        let t = (f32(i) + random(in.world_pos + f32(i) + uniforms.time)) / f32(SAMPLES) * 2. * 3.1415;
        light = light + trace(in.world_pos, vec2<f32>(cos(t), sin(t)), worldPosChange);
    }
    light = 4. * light / f32(SAMPLES);
    light.a = 1.0;
    _OutsideColor = _OutsideColor * light;

    let col = mix(_InsideColor, _OutsideColor, clamp(dist / worldPosChange + 0.5, 0.0, 1.0));

    return col;
}
