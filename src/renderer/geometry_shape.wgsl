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


struct ShapeData {
    data0: vec4<u32>,
    data1: vec4<f32>,
    data2: vec4<f32>,
};

struct ShapesBuffer {
    shapes: array<ShapeData>,
};
@group(1) @binding(0)
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
@group(1) @binding(1)
var<storage, read> bvhBuffer: ShapeBVHNodesBuffer;

struct ShapesConfig {
  numShapes: u32,
  numBvhNodes: u32,
};
@group(1) @binding(2)
var<uniform> shapesConfig: ShapesConfig;

fn wrap(p: vec2<f32>) -> vec2<f32> {
    let s = ceil(abs(p * uniforms.inv_world_size)) + 0.5;
    return (p + s * uniforms.world_size) % uniforms.world_size - 0.5 * uniforms.world_size;
}

fn world_to_depth(z: f32) -> f32 {
    return saturate(-0.25 * z + 0.5);
}

// Vertex shader

struct VertexOutput {
    @location(0) world_pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_index: u32,
    @builtin(position) position: vec4<f32>,
}

@vertex
fn main_vert(@builtin(vertex_index) in_vertex_index: u32, @builtin(instance_index) in_instance_index: u32) -> VertexOutput {
    // triangles `0 1 2`, `2 1 3`
    var vertices: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
        vec2<f32>(-1., 1.),
        vec2<f32>(-1., -1.),
        vec2<f32>(1., 1.),
        vec2<f32>(1., -1.),
    );
    var out: VertexOutput;
    out.instance_index = in_instance_index;
    let shape = shapesBuffer.shapes[in_instance_index];
    var aabb_min: vec3<f32>;
    var aabb_max: vec3<f32>;
    if (shape.data0[0] == 0u) {
        aabb_min = shape.data1.xyz - shape.data1.w;
        aabb_max = shape.data1.xyz + shape.data1.w;
    } else if (shape.data0[0] == 1u) {
        aabb_min = min(shape.data1.xyz - shape.data1.w, shape.data2.xyz - shape.data2.w);
        aabb_max = max(shape.data1.xyz + shape.data1.w, shape.data2.xyz + shape.data2.w);
    }

    let world_pos = 0.5 * (aabb_min + aabb_max).xy;
    let delta = 0.5 * (aabb_max - aabb_min).xy * vertices[in_vertex_index];
    out.world_pos = world_pos + delta;
    let position = 2. * wrap(world_pos - uniforms.translate - uniforms.pixel_size * uniforms.sub_pixel_jitter) / uniforms.view_size;
    let position_delta = 2. * delta / uniforms.view_size;
    out.position = vec4<f32>(position + position_delta, world_to_depth(aabb_max.z), 1.0);
    return out;
}

// Fragment shader

fn wrap3(p: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(wrap(p.xy), p.z);
}

const kMaxRayDistance: f32 = 1e20;

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
    let shape = shapesBuffer.shapes[in.instance_index];
    let ro = vec3<f32>(in.world_pos, 2.0);
    let rd = vec3<f32>(0., 0., -1.);
    let tmax = 4.;
    var normal = vec3<f32>(0., 0., 1.);
    var z = -2.;

    if shape.data0[0] == 0u {
        let oro = wrap3(ro - shape.data1.xyz);
        let t = iSphere(oro, rd, shape.data1.w);
        if t < 0. || t > tmax {
            discard;
        }
        normal = nSphere(oro + t * rd);
        z = (ro + t * rd).z;
    } else if shape.data0[0] == 1u {
        let oro = wrap3(ro - shape.data1.xyz);
        let tnor = iRoundedCone(oro, rd, vec3<f32>(0.), shape.data2.xyz - shape.data1.xyz, shape.data1.w, shape.data2.w);
        if tnor.x < 0. || tnor.x > tmax {
            discard;
        }
        normal = tnor.yzw;
        z = (ro + tnor.x * rd).z;
    }

    let albedo = unpack4x8unorm(shape.data0.y).xyz;
    let params = unpack4x8unorm(shape.data0.z);
    let metallic = params.x;
    let roughness = params.y;

    return FragmentOutput(
        world_to_depth(z),
        vec4<f32>(albedo, 1.0),
        vec4<f32>(encode_normal(normal), metallic, roughness),
    );
}
