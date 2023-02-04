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
    // triangles `0 1 2`, `2 1 3`
    var vertices: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
        vec2<f32>(-1., 1.),
        vec2<f32>(-1., -1.),
        vec2<f32>(1., 1.),
        vec2<f32>(1., -1.),
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

struct ShapeData {
    data0: vec4<u32>,
    data1: vec4<f32>,
    data2: vec4<f32>,
};

struct ShapesBuffer {
    shapes: array<ShapeData>,
};
@group(2) @binding(0)
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
@group(2) @binding(1)
var<storage, read> bvhBuffer: ShapeBVHNodesBuffer;

struct ShapesConfig {
  numShapes: u32,
  numBvhNodes: u32,
};
@group(2) @binding(2)
var<uniform> shapesConfig: ShapesConfig;

fn unpackSdf(v: f32) -> f32 {
    return v;
}

fn sceneDist(world_pos: vec2<f32>) -> f32 {
    var uv = world_pos * uniforms.inv_world_size;
    uv.y = -uv.y;
    uv = uv + 0.5;
    return unpackSdf(textureSample(t_sdf, s_sdf, uv).r);
}

fn wrap(p: vec2<f32>) -> vec2<f32> {
    let s = ceil(abs(p * uniforms.inv_world_size)) + 0.5;
    return (p + s * uniforms.world_size) % uniforms.world_size - 0.5 * uniforms.world_size;
}

fn wrap3(p: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(wrap(p.xy), p.z);
}

fn traceTerrain(ro: vec2<f32>, rd: vec2<f32>, tmax: f32) -> f32 {
    var t: f32 = 0.;
    for (var i: i32 = 0; i < 32; i = i + 1) {
        let h = max(sceneDist(ro + t * rd), 0.);
        t += h;
        if h < .001 {
            return t;
        }
        if t > tmax {
            return tmax;
        }
    }
    return t;
}

let kMaxRayDistance: f32 = 1e20;

fn iAABB(ro: vec3<f32>, inv_rd: vec3<f32>, aabb_rad: vec3<f32>, tmax: f32) -> bool {
    let n = inv_rd * ro;
    let k = abs(inv_rd) * aabb_rad;
    let t1 = -n - k;
    let t2 = -n + k;

    let tnear = max(max(t1.x, t1.y), t1.z);
    let tfar = min(min(t2.x, t2.y), t2.z);

    return tfar > max(tnear, 0.) && tnear < tmax;
}

fn iSphere(ro: vec3<f32>, rd: vec3<f32>, radius: f32) -> f32 {
    let b = dot(rd, ro);
    let c = dot(ro, ro) - (radius * radius);
    let h = b * b - c;
    if h < 0.0 {
        return kMaxRayDistance;
    }
    return -b - sqrt(h);
}

fn nSphere(pos: vec3<f32>) -> vec3<f32> {
    return normalize(pos);
}

// https://www.shadertoy.com/view/MlKfzm
fn iRoundedCone(ro: vec3<f32>, rd: vec3<f32>, pa: vec3<f32>, pb: vec3<f32>, ra: f32, rb: f32) -> vec4<f32> {
    let ba = pb - pa;
    let oa = ro - pa;
    let ob = ro - pb;
    let rr = ra - rb;
    let m0 = dot(ba, ba);
    let m1 = dot(ba, oa);
    let m2 = dot(ba, rd);
    let m3 = dot(rd, oa);
    let m5 = dot(oa, oa);
    let m6 = dot(ob, rd);
    let m7 = dot(ob, ob);

    let d2 = m0 - rr * rr;

    let k2 = d2 - m2 * m2;
    let k1 = d2 * m3 - m1 * m2 + m2 * rr * ra;
    let k0 = d2 * m5 - m1 * m1 + m1 * rr * ra * 2.0 - m0 * ra * ra;

    let h = k1 * k1 - k0 * k2;
    if h < 0.0 {
        return vec4<f32>(kMaxRayDistance);
    }
    var t = (-sqrt(h) - k1) / k2;

    let y = m1 - ra * rr + t * m2;
    if y > 0.0 && y < d2 {
        return vec4<f32>(t, normalize(d2 * (oa + t * rd) - ba * y));
    }

    let h1 = m3 * m3 - m5 + ra * ra;
    let h2 = m6 * m6 - m7 + rb * rb;
    if max(h1, h2) < 0.0 {
        return vec4<f32>(kMaxRayDistance);
    }

    var r = vec4<f32>(kMaxRayDistance);
    if h1 > 0.0 {
        t = -m3 - sqrt(h1);
        r = vec4<f32>(t, (oa + t * rd) / ra);
    }
    if h2 > 0.0 {
        t = -m6 - sqrt(h2);
        if t < r.x {
            r = vec4<f32>(t, (ob + t * rd) / rb);
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
    if s.data0[0] == 0u {
        let oro = wrap3(ro - s.data1.xyz);
        let t = iSphere(oro, rd, s.data1.w);
        if t > 0. && t < result.t {
            return RayTraceResult(t, nSphere(oro + t * rd), shapeIndex);
        }
    } else if s.data0[0] == 1u {
        let oro = wrap3(ro - s.data1.xyz);
        let tnor = iRoundedCone(oro, rd, vec3<f32>(0.), s.data2.xyz - s.data1.xyz, s.data1.w, s.data2.w);
        if tnor.x > 0. && tnor.x < result.t {
            return RayTraceResult(tnor.x, tnor.yzw, shapeIndex);
        }
    }
    return result;
} 

fn traceRayBVH(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> RayTraceResult {
    var result = RayTraceResult(
        tmax,
        vec3<f32>(.0, .0, .0),
        shapesConfig.numShapes,
    );
    var nodeIndex = 0;
    let maxLength = i32(shapesConfig.numBvhNodes);
    let inv_rd = 1.0 / rd;

    while nodeIndex < maxLength {
        let node = bvhBuffer.nodes[nodeIndex];

        if node.entry < 0 {
            result = traceRayShape(u32(-node.entry), ro, rd, result);
            nodeIndex = node.exit;
        } else if iAABB(wrap3(ro - node.aabb_pos.xyz), inv_rd, node.aabb_rad.xyz, result.t) {
            nodeIndex = node.entry;
        } else {
            nodeIndex = node.exit;
        }
    }
    return result;
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

struct FragmentOutput {
    @builtin(frag_depth) depth: f32,
    @location(0) albedo: vec4<f32>,
    @location(1) normals_metallic_roughness: vec4<f32>,
}

fn encode_normal(normal: vec3<f32>) -> vec2<f32> {
    // Project the sphere onto the octahedron (|x|+|y|+|z| = 1) and then onto the xy-plane
    let invL1Norm = 1.0 / (abs(normal.x) + abs(normal.y) + abs(normal.z));
    var p = normal.xy * invL1Norm;

    // Wrap the octahedral faces from the negative-Z space
    if (normal.z < 0.) {
        p = (1.0 - abs(p.yx)) * mix(vec2(-1.0), vec2(1.0), step(vec2<f32>(0.), p.xy));
    }

    // Convert to [0..1]
    return saturate(p.xy * 0.5 + 0.5);
}

@fragment
fn main_frag(in: VertexOutput) -> FragmentOutput {
    let dist = sceneDist(in.world_pos);
    let RO = vec3<f32>(in.world_pos, 2.0);
    let RD = vec3<f32>(0., 0., -1.);
    var N = vec3<f32>(0., 0., 1.);
    var WorldPos = vec3<f32>(in.world_pos, -2.);

    var albedo: vec3<f32>;
    var metallic: f32;
    var roughness: f32;

    if dist < 0. {
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

    if dist > 0. {
        let result = traceRayBVH(RO, RD, RO.z - WorldPos.z);
        if result.shapeIndex < shapesConfig.numShapes {
            N = result.normal;
            WorldPos = RO + result.t * RD;
            let shape = shapesBuffer.shapes[result.shapeIndex];
            albedo = unpack4x8unorm(shape.data0.y).xyz;
            let params = unpack4x8unorm(shape.data0.z);
            metallic = params.x;
            roughness = params.y;
        }
    }

    return FragmentOutput(
        saturate(-0.25 * WorldPos.z),
        vec4<f32>(albedo, 1.0),
        vec4<f32>(encode_normal(N), metallic, roughness),
    );
}
