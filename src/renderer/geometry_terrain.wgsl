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

fn unpackSdf(v: f32) -> f32 {
    return v;
}

fn sceneDist(world_pos: vec2<f32>) -> f32 {
    var uv = world_pos * uniforms.inv_world_size;
    uv.y = -uv.y;
    uv = uv + 0.5;
    return unpackSdf(textureSample(t_sdf, s_sdf, uv).r);
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
    let normal = vec3<f32>(0., 0., 1.);

    var albedo: vec3<f32>;
    var metallic: f32;
    var roughness: f32;
    var depth: f32;

    if dist < 0. {
        albedo = vec3<f32>(0., 0., 0.);
        metallic = 0.;
        roughness = 1.;
        depth = 0.;
    } else {
        albedo = vec3<f32>(.5, .5, .5);
        metallic = 0.;
        roughness = 0.1;
        depth = 1.;
    }
    let patternMask = clamp(dot(floor((abs(in.world_pos) + .5) / 1.0), vec2<f32>(1.0)) % 2.0, 0.8, 1.0);
    albedo = albedo * patternMask;

    return FragmentOutput(
        depth,
        vec4<f32>(albedo, 1.0),
        vec4<f32>(encode_normal(normal), metallic, roughness),
    );
}
