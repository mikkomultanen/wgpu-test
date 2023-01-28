//----------------------------------------------------------------------------------------
//  1 out, 1 in...
//fn hash11(v: f32) -> f32
//{
//    var p: f32 = fract(v * .1031);
//    p = p * (p + 33.33);
//    p = p * (p + p);
//    return fract(p);
//}

//----------------------------------------------------------------------------------------
//  1 out, 2 in...
//fn hash12(p: vec2<f32>) -> f32
//{
//	var p3: vec3<f32> = fract(vec3<f32>(p.xyx) * .1031);
//    p3 = p3 + dot(p3, p3.yzx + 33.33);
//    return fract((p3.x + p3.y) * p3.z);
//}

//----------------------------------------------------------------------------------------
//  1 out, 3 in...
//fn hash13(p: vec3<f32>) -> f32
//{
//	var p3 = fract(p * .1031);
//    p3 = p3 + dot(p3, p3.zyx + 31.32);
//    return fract((p3.x + p3.y) * p3.z);
//}

//----------------------------------------------------------------------------------------
//  2 out, 1 in...
//fn hash21(p: f32) -> vec2<f32>
//{
//	var p3: vec3<f32> = fract(vec3<f32>(p) * vec3<f32>(.1031, .1030, .0973));
//	p3 = p3 + dot(p3, p3.yzx + 33.33);
//    return fract((p3.xx+p3.yz)*p3.zy);
//}

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

struct ShapeData {
    data0: vec4<u32>,
    data1: vec4<f32>,
    data2: vec4<f32>,
};

struct ShapesBuffer {
    shapes: array<ShapeData>,
};
@group(3) @binding(0)
var<storage, read> shapesBuffer: ShapesBuffer;

struct ShapeBVHNode {
    aabb_pos: vec3<f32>,
    entry: i32,
    aabb_rad: vec3<f32>,
    exit: i32,
}

struct ShapeBVHNodesBuffer {
    nodes: array<ShapeBVHNode>,
};
@group(3) @binding(1)
var<storage, read> bvhBuffer: ShapeBVHNodesBuffer;

struct ShapesConfig {
  numShapes: u32,
  numBvhNodes: u32,
};
@group(3) @binding(2)
var<uniform> shapesConfig: ShapesConfig;

@group(4) @binding(0)
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

fn hardShadow(ro: vec2<f32>, rd: vec2<f32>, tmax: f32, radius: f32) -> f32 {
    if (tmax < radius) {
        return 1.;
    }
    var t: f32 = 0.0;
    for(var i: i32 = 0; i < 32; i = i + 1) {
        let h = sceneDist(ro + t * rd);
        if( h < .001) {
            return 0.;
        }            
        t += h;
        if(t > tmax - radius) {
            return 1.;
        }
    }
    return 0.;
}

fn softShadow(ro: vec2<f32>, rd: vec2<f32>, tmax: f32, radius: f32) -> f32 {
    if (tmax <= radius) {
        return 1.;
    }
    var r: f32 = 1.0;
    var t: f32 = 0.02;
    var ph: f32 = 1.0e20;
    let k = radius / tmax;
    //let k = radius * inverseSqrt(lightDistance * lightDistance - radius * radius);
    for(var i: i32 = 0; i < 64; i = i + 1) {
        let extra = t * k;
        let h = sceneDist(ro + t * rd) + extra;
        if( h < .001) {
            return 0.;
        }
        if(t + h - extra > tmax - radius) {
            return r;
        }
        let y = h*h/(2.0*ph);
        r = min(r, sqrt(h*h-y*y)/max(0.0,(t - y)*(k + k)));
        ph = h;
        t += .5 * h;
    }
    return 0.;
}

fn wrap(p: vec2<f32>) -> vec2<f32> 
{
    let s = ceil(abs(p * uniforms.inv_world_size)) + 0.5;
    return (p + s * uniforms.world_size) % uniforms.world_size - 0.5 * uniforms.world_size;
}

fn wrap3(p: vec3<f32>) -> vec3<f32>
{
    return vec3<f32>(wrap(p.xy), p.z);
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

let PI: f32 = 3.14159265359;
let TwoPI: f32 = 6.28318530718;

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

fn traceTerrain(ro: vec2<f32>, rd: vec2<f32>, tmax: f32) -> f32 {
    var t: f32 = 0.;
    for(var i: i32 = 0; i < 32; i = i + 1) {
        let h = max(sceneDist(ro + t * rd), 0.);
        t += h;
        if( h < .001) {
            return t;
        }            
        if(t > tmax) {
            return tmax;
        }
    }
    return t;
}

let kMaxRayDistance: f32 = 1e20;

fn iAABB(ro: vec3<f32>, inv_rd: vec3<f32>, aabb_rad: vec3<f32>, tmax: f32) -> bool {
    let n = inv_rd*ro;
    let k = abs(inv_rd)*aabb_rad;
    let t1 = -n - k;
    let t2 = -n + k;

    let tnear = max( max( t1.x, t1.y ), t1.z );
    let tfar = min( min( t2.x, t2.y ), t2.z );
	
    return tfar > max(tnear, 0.) && tnear < tmax;
}

fn iSphere(ro: vec3<f32>, rd: vec3<f32>, radius: f32) -> f32 {
    let b = dot(rd, ro);
    let c = dot(ro, ro) - (radius * radius);
    let h = b*b - c;
    if (h < 0.0) {
        return kMaxRayDistance;
    }
    return -b - sqrt(h);
}

fn nSphere(pos: vec3<f32>) -> vec3<f32> {
    return normalize(pos);
}

// https://www.shadertoy.com/view/MlKfzm
fn iRoundedCone(ro: vec3<f32>, rd: vec3<f32>, 
                  pa: vec3<f32>, pb: vec3<f32>,
                  ra: f32, rb: f32) -> vec4<f32> {
    let ba = pb - pa;
	let oa = ro - pa;
	let ob = ro - pb;
    let rr = ra - rb;
    let m0 = dot(ba,ba);
    let m1 = dot(ba,oa);
    let m2 = dot(ba,rd);
    let m3 = dot(rd,oa);
    let m5 = dot(oa,oa);
	let m6 = dot(ob,rd);
    let m7 = dot(ob,ob);
    
    let d2 = m0-rr*rr;
    
	let k2 = d2    - m2*m2;
    let k1 = d2*m3 - m1*m2 + m2*rr*ra;
    let k0 = d2*m5 - m1*m1 + m1*rr*ra*2.0 - m0*ra*ra;
    
	let h = k1*k1 - k0*k2;
	if(h < 0.0) {
        return vec4<f32>(kMaxRayDistance);
    }
    var t = (-sqrt(h)-k1)/k2;

    let y = m1 - ra*rr + t*m2;
    if( y>0.0 && y<d2 ) 
    {
        return vec4<f32>(t, normalize( d2*(oa + t*rd)-ba*y) );
    }

    let h1 = m3*m3 - m5 + ra*ra;
    let h2 = m6*m6 - m7 + rb*rb;
    if( max(h1,h2)<0.0 ) {
        return vec4<f32>(kMaxRayDistance);
    }
    
    var r = vec4<f32>(kMaxRayDistance);
    if( h1>0.0 )
    {        
    	t = -m3 - sqrt( h1 );
        r = vec4<f32>( t, (oa+t*rd)/ra );
    }
	if( h2>0.0 )
    {
    	t = -m6 - sqrt( h2 );
        if( t<r.x ) {
            r = vec4<f32>( t, (ob+t*rd)/rb );
        }        
    }
    
    return r;
}

struct RayTraceResult {
    t: f32,
    normal: vec3<f32>,
    shapeIndex: u32,
}

fn traceRayShape(shapeIndex: u32, ro: vec3<f32>, rd: vec3<f32>, result: RayTraceResult) -> RayTraceResult {
    let s = shapesBuffer.shapes[shapeIndex];
    if (s.data0[0] == 0u) {
        let oro = wrap3(ro - s.data1.xyz);
        let t = iSphere(oro, rd, s.data1.w);
        if (t > 0. && t < result.t) {
            return RayTraceResult(t, nSphere(oro + t * rd), shapeIndex);
        }
    } else if (s.data0[0] == 1u) {
        let oro = wrap3(ro - s.data1.xyz);
        let tnor = iRoundedCone(oro, rd, vec3<f32>(0.), s.data2.xyz- s.data1.xyz, s.data1.w, s.data2.w);
        if (tnor.x > 0. && tnor.x < result.t) {
            return RayTraceResult(tnor.x, tnor.yzw, shapeIndex);
        }
    }
    return result;
} 

fn traceOccShape(shapeIndex: u32, ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> bool {
    let s = shapesBuffer.shapes[shapeIndex];
    if (s.data0[0] == 0u) {
        let oro = wrap3(ro - s.data1.xyz);
        let t = iSphere(oro, rd, s.data1.w);
        if (t > 0. && t < tmax) {
            return true;
        }
    } else if (s.data0[0] == 1u) {
        let oro = wrap3(ro - s.data1.xyz);
        let t = iRoundedCone(oro, rd, vec3<f32>(0.), s.data2.xyz- s.data1.xyz, s.data1.w, s.data2.w).x;
        if (t > 0. && t < tmax) {
            return true;
        }
    }
    return false;
} 

fn traceRayBVH(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> RayTraceResult {
    var result = RayTraceResult(
        tmax,
        vec3<f32>(.0, .0, .0),
        shapesConfig.numShapes,
    );
    var nodeIndex = 0;
    let maxLength = i32(shapesConfig.numBvhNodes);
    let inv_rd = 1.0/rd;

    while (nodeIndex < maxLength) {
        let node = bvhBuffer.nodes[nodeIndex];

        if (node.entry < 0) {
            result = traceRayShape(u32(-node.entry), ro, rd, result);
            nodeIndex = node.exit;
        } else if (iAABB(wrap3(ro - node.aabb_pos.xyz), inv_rd, node.aabb_rad.xyz, result.t)) {
            nodeIndex = node.entry;
        } else {
            nodeIndex = node.exit;
        }
    }
    return result;
}

fn traceOccBVH(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> f32 {
    var nodeIndex = 0;
    let maxLength = i32(shapesConfig.numBvhNodes);
    let inv_rd = 1.0/rd;

    while (nodeIndex < maxLength) {
        let node = bvhBuffer.nodes[nodeIndex];

        if (node.entry < 0) {
            if (traceOccShape(u32(-node.entry), ro, rd, tmax)) {
                return 0.;
            }
            nodeIndex = node.exit;
        } else if (iAABB(wrap3(ro - node.aabb_pos.xyz), inv_rd, node.aabb_rad.xyz, tmax)) {
            nodeIndex = node.entry;
        } else {
            nodeIndex = node.exit;
        }
    }
    return 1.;
}

fn traceRay(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> RayTraceResult {
    var result = RayTraceResult(
        tmax,
        vec3<f32>(.0, .0, .0),
        shapesConfig.numShapes,
    );
    for (var i = 0u; i < shapesConfig.numShapes; i = i + 1u) {
        result = traceRayShape(i, ro, rd, result);
    }
    return result;
}

fn traceOcc(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> f32 {
    for (var i = 0u; i < shapesConfig.numShapes; i++) {
        if (traceOccShape(i, ro, rd, tmax)) {
            return 0.;
        }
    }
    return 1.;
}

fn constructONBfrisvad(normal: vec3<f32>) -> mat3x3<f32> {
    if (normal.z < -0.999805696f) {
        return mat3x3<f32>(
            vec3<f32>(0.0f, -1.0f, 0.0f),
            normal,
            vec3<f32>(-1.0f, 0.0f, 0.0f)
        );
    }
    let a = 1.0f / (1.0f + normal.z);
    let b = -normal.x * normal.y * a;
    return mat3x3<f32>(
        vec3<f32>(1.0f - normal.x * normal.x * a, b, -normal.x),
        normal,
        vec3<f32>(b, 1.0f - normal.y * normal.y * a, -normal.y)
    );
}


@fragment
fn main_frag_pbr(in: VertexOutput) -> @location(0) vec4<f32> {
    let offset = vec3<f32>(0.5 * uniforms.pixel_size.xy, 0.);

    let dist = sceneDist(in.world_pos);
    let RO = vec3<f32>(in.world_pos, 2.0);
    let RD = vec3<f32>(0., 0., -1.);
    var N = vec3<f32>(0., 0., 1.);
    var WorldPos = vec3<f32>(in.world_pos, -2.);

    var albedo: vec3<f32>;
    var metallic: f32;
    var roughness: f32;

    if (dist < 0.) {
        albedo = vec3<f32>(1.0, 0.4, 0.0);
        metallic = 0.;
        roughness = 1.;
    } else {
        albedo = vec3<f32>(1., 1., 1.);
        metallic = 1.;
        roughness = 0.5;
    }
    let patternMask = clamp(dot(floor((abs(in.world_pos) + .5) / 1.0), vec2<f32>(1.0)) % 2.0, 0.8, 1.0);
    albedo = albedo * patternMask;

    if (dist > 0.) {
        let result = traceRayBVH(RO, RD, RO.z - WorldPos.z);
        if (result.shapeIndex < shapesConfig.numShapes) {
            N = result.normal;
            WorldPos = RO + result.t * RD;
            let shape = shapesBuffer.shapes[result.shapeIndex];
            albedo = unpack4x8unorm(shape.data0.y).xyz;
            let params = unpack4x8unorm(shape.data0.z);
            metallic = params.x;
            roughness = params.y;
        }
    }

    let ao = 1.0;

    let F0 = mix(vec3<f32>(.04, .04, .04), albedo, metallic);
	           
    // reflectance equation
    var Lo = vec3<f32>(0., 0., 0.);

    let NdotNegRDx4 = max(dot(N, -RD), 0.) * 4.;

    for (var i = 0u; i < lightsConfig.numLights; i = i + 1u) {
        let light = lightsBuffer.lights[i];
        // calculate per-light radiance
        let l = vec3<f32>(wrap(light.position - WorldPos.xy), 0. - WorldPos.z);

        let r = reflect(RD, N);
        let centerToRay = (dot(l, r) * r) - l;
        let closestPoint = l + centerToRay * clamp(light.radius / length(centerToRay), 0., 1.);
        let distance = length(closestPoint);
        let L = closestPoint * (1. / distance);
        let NdotL = dot(N, L);
        if (NdotL <= 0.) {
            continue;
        }

        let effectiveRange = max(light.range - light.radius, 0.);
        if (distance > effectiveRange) {
            continue;
        }
        let falloff = pow(clamp(1. - pow(distance/effectiveRange, 4.), 0., 1.), 2.) / ((distance * distance) + 1.);
        var shadow = 1.;
        if (distance > light.radius) {
            let distanceToCenter = length(l);
            let invDistanceToCenter = 1. / distanceToCenter;
            let w = l * invDistanceToCenter;

            let toWorld = constructONBfrisvad(w);
            let rand = blue_noise(in.position.xy);
            var q = light.radius * invDistanceToCenter;
            q = sqrt(1.0 - q * q);
            let theta = acos(1. - rand.x + rand.x * q);
            let phi = TwoPI * rand.y;
            let wp = toWorld * vec3<f32>(sin(theta) * cos(phi), cos(theta), sin(theta) * sin(phi));
            let tmax = min(iSphere(-l, wp, light.radius), q * distanceToCenter);

            let distanceToTerrainFromLight = traceTerrain(wrap(WorldPos.xy + tmax * wp.xy), -wp.xy, tmax);
            shadow = mix(
                smoothstep(10., 0., tmax - distanceToTerrainFromLight), // Inside terrain
                step(tmax, distanceToTerrainFromLight), 
                step(0., dist)
            );
            if (shadow == 0.) {
                continue;
            }
            if (dist > 0.) {
                shadow = shadow * traceOccBVH(WorldPos, wp, tmax);
            }
        }
        if (shadow == 0.) {
            continue;
        }
        let radiance = light.color.rgb * shadow * falloff;

        let H = normalize(-RD + L);
        
        // cook-torrance brdf
        let NDF = DistributionGGX(N, H, roughness, distance, light.radius);
        let G   = GeometrySmith(N, -RD, L, roughness);      
        let F   = fresnelSchlick(max(dot(H, -RD), 0.0), F0);       
        
        let kS = F;
        let kD = (vec3<f32>(1., 1., 1.) - kS) * (1.0 - metallic);
        
        let numerator    = NDF * G * F;
        let denominator  = NdotNegRDx4 * NdotL + 0.0001;
        let specular     = numerator / denominator;  
            
        // add to outgoing radiance Lo
        Lo = Lo + (kD  / PI + specular) * radiance * NdotL; 
    }

    let ambient = vec3<f32>(.0, .0, .0) * ao;
    var color: vec3<f32> = ambient + Lo;
    color = color * albedo;
	
    return vec4<f32>(color, 1.0);
}
