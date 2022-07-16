//----------------------------------------------------------------------------------------
//  1 out, 1 in...
fn hash11(v: f32) -> f32
{
    var p: f32 = fract(v * .1031);
    p = p * (p + 33.33);
    p = p * (p + p);
    return fract(p);
}

//----------------------------------------------------------------------------------------
//  1 out, 2 in...
fn hash12(p: vec2<f32>) -> f32
{
	var p3: vec3<f32> = fract(vec3<f32>(p.xyx) * .1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

//----------------------------------------------------------------------------------------
//  1 out, 3 in...
fn hash13(p: vec3<f32>) -> f32
{
	var p3 = fract(p * .1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

//----------------------------------------------------------------------------------------
//  2 out, 1 in...
fn hash21(p: f32) -> vec2<f32>
{
	var p3: vec3<f32> = fract(vec3<f32>(p) * vec3<f32>(.1031, .1030, .0973));
	p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.xx+p3.yz)*p3.zy);
}

//----------------------------------------------------------------------------------------
///  2 out, 2 in...
//vec2 hash22(vec2 p)
//{
//	vec3 p3 = fract(vec3(p.xyx) * vec3(.1031, .1030, .0973));
//    p3 += dot(p3, p3.yzx+33.33);
//    return fract((p3.xx+p3.yz)*p3.zy);
//}

//----------------------------------------------------------------------------------------
///  2 out, 3 in...
//vec2 hash23(vec3 p3)
//{
//	p3 = fract(p3 * vec3(.1031, .1030, .0973));
//    p3 += dot(p3, p3.yzx+33.33);
//    return fract((p3.xx+p3.yz)*p3.zy);
//}

//----------------------------------------------------------------------------------------
//  3 out, 1 in...
//vec3 hash31(float p)
//{
//   vec3 p3 = fract(vec3(p) * vec3(.1031, .1030, .0973));
//   p3 += dot(p3, p3.yzx+33.33);
//   return fract((p3.xxy+p3.yzz)*p3.zyx); 
//}


//----------------------------------------------------------------------------------------
///  3 out, 2 in...
//vec3 hash32(vec2 p)
//{
//	vec3 p3 = fract(vec3(p.xyx) * vec3(.1031, .1030, .0973));
//    p3 += dot(p3, p3.yxz+33.33);
//    return fract((p3.xxy+p3.yzz)*p3.zyx);
//}

//----------------------------------------------------------------------------------------
///  3 out, 3 in...
//vec3 hash33(vec3 p3)
//{
//	p3 = fract(p3 * vec3(.1031, .1030, .0973));
//    p3 += dot(p3, p3.yxz+33.33);
//    return fract((p3.xxy + p3.yxx)*p3.zyx);
//}

//----------------------------------------------------------------------------------------
// 4 out, 1 in...
//vec4 hash41(float p)
//{
//	vec4 p4 = fract(vec4(p) * vec4(.1031, .1030, .0973, .1099));
//    p4 += dot(p4, p4.wzxy+33.33);
//    return fract((p4.xxyz+p4.yzzw)*p4.zywx);  
//}

//----------------------------------------------------------------------------------------
// 4 out, 2 in...
//vec4 hash42(vec2 p)
//{
//	vec4 p4 = fract(vec4(p.xyxy) * vec4(.1031, .1030, .0973, .1099));
//    p4 += dot(p4, p4.wzxy+33.33);
//    return fract((p4.xxyz+p4.yzzw)*p4.zywx);
//}

//----------------------------------------------------------------------------------------
// 4 out, 3 in...
//vec4 hash43(vec3 p)
//{
//	vec4 p4 = fract(vec4(p.xyzx)  * vec4(.1031, .1030, .0973, .1099));
//    p4 += dot(p4, p4.wzxy+33.33);
//    return fract((p4.xxyz+p4.yzzw)*p4.zywx);
//}

//----------------------------------------------------------------------------------------
// 4 out, 4 in...
//vec4 hash44(vec4 p4)
//{
//	p4 = fract(p4  * vec4(.1031, .1030, .0973, .1099));
//    p4 += dot(p4, p4.wzxy+33.33);
//    return fract((p4.xxyz+p4.yzzw)*p4.zywx);
//}


struct Uniforms {
    translate: vec2<f32>,
    view_size: vec2<f32>,
    world_size: vec2<f32>,
    inv_world_size: vec2<f32>,
    pixel_size: vec2<f32>,
    sub_pixel_jitter: vec2<f32>,
    mouse: vec2<f32>,
    cursor_size: f32,
    time: f32,
    exposure: f32,
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
    out.world_pos = uniforms.translate + 0.5 * out.position.xy * uniforms.view_size + uniforms.pixel_size * uniforms.sub_pixel_jitter;
    return out;
}

// Fragment shader

@group(1) @binding(0)
var t_sdf: texture_2d<f32>;
@group(1) @binding(1)
var s_sdf: sampler;

struct LightData {
    color: vec4<f32>,
    position: vec2<f32>,
    radius: f32,
    range: f32,
};

struct LightsBuffer {
    lights: array<LightData>,
};
@group(2) @binding(0)
var<storage, read> lightsBuffer: LightsBuffer;

struct LightsConfig {
  numLights : u32,
};
@group(2) @binding(1)
var<uniform> lightsConfig: LightsConfig;

@group(3) @binding(0)
var t_blue_noise: texture_2d<f32>;

fn unpackSdf(v: f32) -> f32 {
    return v;
}

fn sceneDist(world_pos: vec2<f32>) -> f32 {
    var uv = world_pos * uniforms.inv_world_size;
    uv.y = -uv.y;
    uv = uv + 0.5;
    return unpackSdf(textureSample(t_sdf, s_sdf, uv).r);
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
        r = min(r, t/max(0.0,(d - y)*(k + k)));
        ph = h;
        d = d + .5 * h;
    }
    return 0.;
}

fn wrap(p: vec2<f32>) -> vec2<f32> 
{
    let s = ceil(abs(p / uniforms.world_size)) + 0.5;
    return (p + s * uniforms.world_size) % uniforms.world_size - 0.5 * uniforms.world_size;
}

fn drawLight(p: vec2<f32>, pos: vec2<f32>, color: vec3<f32>, dist: f32, range: f32, radius: f32, pChange: f32) -> vec3<f32>
{
    if (dist < 0.) {
        return vec3<f32>(0., 0., 0.);
    }

    let d = wrap(pos - p);
	// distance to light
	let ld = length(d);
	
	// out of range
	if (ld > range) {
        return vec3<f32>(0., 0., 0.);
    }
	
	// shadow and falloff
	let shad = softShadow(p, d / max(radius, ld), ld, radius);
	var fall = (range - ld + radius) / range;
	fall = fall * fall;
	let source = 1.0 - clamp((ld - radius) / pChange + 0.5, 0.0, 1.0);
    return mix(shad * fall, 4., source) * color;
}

fn rotation(angle: f32) -> mat2x2<f32> {
    let cs = cos(angle);
    let sn = sin(angle);
    return mat2x2<f32>(vec2<f32>(cs, -sn), vec2<f32>(sn, cs));
}

fn perpendicular(v: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(v.y, -v.x);
}

fn blue_noise(p: vec2<f32>) -> vec4<f32> {
    let dimensions = textureDimensions(t_blue_noise);
    let coords = vec2<i32>(p) % dimensions;
    return textureLoad(t_blue_noise, coords, 0);
}

fn traceShadow(p_screen: vec2<f32>, p: vec2<f32>, pos: vec2<f32>, lightDir: vec2<f32>, lightDistance: f32, radius: f32) -> f32 {
    if (lightDistance <= radius) {
        return 0.;
    }
    let r2 = radius * radius;
    let a = r2 / lightDistance;
    let rand = 2. * blue_noise(p_screen).r - 1.;
    let b = sqrt(r2 - a * a) * rand * 0.9999;

    let ab = a * lightDir + b * perpendicular(lightDir);
    let ab_d = wrap(p - pos - ab);

    let xa = dot(ab_d, ab_d);
    let xb = 2. * dot(ab, ab_d);
    let xc = dot(ab, ab) - r2;
    let x = (-xb + sqrt(xb * xb - 4. * xa * xc)) / (2. * xa);

    let lp = pos + ab + x * ab_d;

    var d: f32 = 0.;
    let ld = wrap(p - lp);
    let lld = length(ld);
    let rayDir = ld / lld;
    for(var i: i32 = 0; i < 32; i = i + 1) {
        let h = max(sceneDist(lp + d * rayDir), 0.);
        if( h < .001) {
            return lld - d - h;
        }            
        d = d + h;
        if(d > lld) {
            return lld - d;
        }
    }
    return lld - d;
}

fn traceLight(p_screen: vec2<f32>, p: vec2<f32>, pos: vec2<f32>, color: vec3<f32>, dist: f32, range: f32, radius: f32, pChange: f32) -> vec3<f32>
{
    if (dist < 0.) {
        return vec3<f32>(0., 0., 0.);
    }

    let d = wrap(p - pos);
	// distance to light
	let ld = length(d);
	
	// out of range
	if (ld > range) {
        return vec3<f32>(0., 0., 0.);
    }
	
	// shadow and falloff
    let lightDir = d / max(radius, ld);
	let shad = step(traceShadow(p_screen, p, pos, lightDir, ld, radius), 0.);
	var fall = (range - ld + radius) / range;
	fall = fall * fall;
	let source = 1.0 - clamp((ld - radius) / pChange + 0.5, 0.0, 1.0);
    return mix(shad * fall, 4., source) * color;
}

@fragment
fn main_frag(in: VertexOutput) -> @location(0) vec4<f32> {
    let worldPosChange = fwidth(in.world_pos.x);

    let dist = sceneDist(in.world_pos);

    var col = vec3<f32>(0., 0., 0.);
    for (var i = 0u; i < lightsConfig.numLights; i = i + 1u) {
        let light = lightsBuffer.lights[i];
        col = col + traceLight(in.position.xy, in.world_pos, light.position, light.color.rgb, dist, light.range, light.radius, worldPosChange);
    }

    let InsideColor = vec3<f32>(1.0, 0.4, 0.0);
    let OutsideColor = vec3<f32>(0.5, 0.5, 0.5);
    if (dist < 0.) {
        col = col * InsideColor;
    } else {
        col = col * OutsideColor;
    }

    return vec4<f32>(col, 1.);
}

let PI: f32 = 3.14159265359;

fn DistributionGGX(N: vec3<f32>, H: vec3<f32>, roughness: f32, distance: f32, radius: f32) -> f32
{
    let a      = roughness*roughness;
    let aPrime = clamp(radius/(distance * 2.) + a, 0., 1.);
    let a2     = a*aPrime;
//    let aPrime = a / clamp(radius/(distance * 2.) + a, 0., 1.);
//    let a2     = aPrime * aPrime;
    let NdotH  = max(dot(N, H), 0.0);
    let NdotH2 = NdotH*NdotH;
	
    let num   = a2;
    var denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
	
    return num / denom;
}

fn GeometrySchlickGGX(NdotV: f32, roughness: f32) -> f32
{
    let r = (roughness + 1.0);
    let k = (r*r) / 8.0;

    let num   = NdotV;
    let denom = NdotV * (1.0 - k) + k;
	
    return num / denom;
}

fn GeometrySmith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32
{
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx2  = GeometrySchlickGGX(NdotV, roughness);
    let ggx1  = GeometrySchlickGGX(NdotL, roughness);
	
    return ggx1 * ggx2;
}

fn fresnelSchlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32>
{
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

let V: vec3<f32> = vec3<f32>(0., 0., -1.);

fn shadow_pbr(p_screen: vec2<f32>, p: vec2<f32>, pos: vec2<f32>, radius: f32) -> f32 {
    let d = wrap(p - pos);
	// distance to light
	let ld = length(d);
	
	// shadow and falloff
    let lightDir = d / max(radius, ld);
	return traceShadow(p_screen, p, pos, lightDir, ld, radius);
}

@fragment
fn main_frag_pbr(in: VertexOutput) -> @location(0) vec4<f32> {
    let offset = vec3<f32>(0.5 * uniforms.pixel_size.xy, 0.);

    let dist = sceneDist(in.world_pos);
//    let distX = sceneDist(in.world_pos + offset.xz);
//    let distY = sceneDist(in.world_pos + offset.zy);
//    let distN = vec2<f32>((distX - dist) / offset.x, (distY - dist) / offset.y);
//    let t = clamp(0.5 * dist + 1., 0., 1.);
//    let N = normalize(vec3<f32>((6. * t * (1. - t)) * distN, -1.));
//    let WorldPos = vec3<f32>(in.world_pos, 2. * smoothStep(0., 1., t));
//    let N = normalize(vec3<f32>(((2. * t) * step(1., t) * step(t, 0.)) * distN, -1.));
//    let WorldPos = vec3<f32>(in.world_pos, 2. * t * t);
//    let N = mix(normalize(vec3<f32>(distN, -1.)), vec3<f32>(0., 0., -1.), step(-dist, 0.));
//    let WorldPos = vec3<f32>(in.world_pos, 2. * step(-dist, 0.));
    let N = vec3<f32>(0., 0., -1.);
    let WorldPos = vec3<f32>(in.world_pos, 2.);

    var albedo: vec3<f32>;
    if (dist < 0.) {
        albedo = vec3<f32>(1.0, 0.4, 0.0);
    } else {
        albedo = vec3<f32>(1., 1., 1.);
    }
    let patternMask = clamp(dot(floor((abs(in.world_pos) + 2.0) / 4.0), vec2<f32>(1.0)) % 2.0, 0.8, 1.0);
    albedo = albedo * patternMask;
    let metallic = 0.;
    let roughness = .5;
    let ao = 1.0;

    var F0 = vec3<f32>(.04, .04, .04); 
    F0 = mix(F0, albedo, metallic);
	           
    // reflectance equation
    var Lo = vec3<f32>(0., 0., 0.);

    for (var i = 0u; i < lightsConfig.numLights; i = i + 1u) {
        let light = lightsBuffer.lights[i];
        // calculate per-light radiance
        let l = vec3<f32>(wrap(light.position - WorldPos.xy), 0. - WorldPos.z);

        let r = reflect(-V, N);
        let centerToRay = (dot(l, r) * r) - l;
        let centerToRayLength = length(centerToRay);
        let closestPoint = l + centerToRay * clamp(light.radius / length(centerToRay), 0., 1.);
        let L = normalize(closestPoint);
        let distance = length(closestPoint);

        let effectiveRange = max(light.range - light.radius, 0.);
        if (distance > effectiveRange) {
            continue;
        }
        //let falloff = pow(clamp(1. - pow(distance/effectiveRange, 4.), 0., 1.), 2.) / ((distance * distance) + 1.);
        let falloff = pow((effectiveRange - distance) / effectiveRange, 2.);
        let distanceToSurface = shadow_pbr(in.position.xy, WorldPos.xy, light.position, light.radius);
        let shadow = mix(
            smoothstep(10., 0., distanceToSurface), 
            step(distanceToSurface, 0.), 
            step(0., dist)
        );
        let radiance = light.color.rgb * shadow * falloff;

        //let attenuation = 1.0 / (distance * distance);
        //let radiance     = light.color.rgb * attenuation;        
        let H = normalize(V + L);
        
        // cook-torrance brdf
        let NDF = DistributionGGX(N, H, roughness, distance, light.radius);
        let G   = GeometrySmith(N, V, L, roughness);      
        let F   = fresnelSchlick(max(dot(H, V), 0.0), F0);       
        
        let kS = F;
        let kD = (vec3<f32>(1., 1., 1.) - kS) * (1.0 - metallic);
        
        let numerator    = NDF * G * F;
        let denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
        let specular     = numerator / denominator;  
            
        // add to outgoing radiance Lo
        let NdotL = max(dot(N, L), 0.0);                

        Lo = Lo + (kD  / PI + specular) * radiance * NdotL; 
    }

    let ambient = vec3<f32>(.0, .0, .0) * ao;
    var color: vec3<f32> = ambient + Lo;
    color = color * albedo;
	
    return vec4<f32>(color, 1.0);
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
    for(var i: i32 = 0; i < 32; i = i + 1) {
        let l = lightDist(d) + 1.;
        if( l - 1. < worldPosChange) {
            let fall = (range - dl - l + 10.0) / range;
            return vec4<f32>(0.75, 1.0, 0.5, 1.0) * fall;
        }
        let h = sceneDist(d);
        if( h < worldPosChange) {
            return vec4<f32>(0., 0., 0., 1.);
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

@fragment
fn main_frag_gi(in: VertexOutput) -> @location(0) vec4<f32> {
    let worldPosChange = fwidth(in.world_pos.x);

    var light = vec4<f32>(0., 0., 0., 1.);
    let rand = blue_noise(in.position.xy);
    var t = (0. + rand.r) / 4. * 2. * 3.1415;
    light = light + trace(in.world_pos, vec2<f32>(cos(t), sin(t)), worldPosChange);
    t = (1. + rand.g) / 4. * 2. * 3.1415;
    light = light + trace(in.world_pos, vec2<f32>(cos(t), sin(t)), worldPosChange);
    t = (2. + rand.b) / 4. * 2. * 3.1415;
    light = light + trace(in.world_pos, vec2<f32>(cos(t), sin(t)), worldPosChange);
    t = (3. + rand.a) / 4. * 2. * 3.1415;
    light = light + trace(in.world_pos, vec2<f32>(cos(t), sin(t)), worldPosChange);
    light = 4. * light / 4.;
    light.a = 1.0;

    return light;
}
