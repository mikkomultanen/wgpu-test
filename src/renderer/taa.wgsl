[[block]]
struct Uniforms {
    translate: vec2<f32>;
    view_size: vec2<f32>;
    world_size: vec2<f32>;
    inv_world_size: vec2<f32>;
    pixel_size: vec2<f32>;
    sub_pixel_jitter: vec2<f32>;
    mouse: vec2<f32>;
    cursor_size: f32;
    time: f32;
    exposure: f32;
};

let flt_taa_anti_sparkle = 0.1; // TODO move to uniforms
let flt_taa_variance = 0.2; // TODO move to uniforms

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

[[group(1), binding(0)]]
var t_colorTexture: texture_2d<f32>;

[[group(2), binding(0)]]
var t_historyTexture: texture_2d<f32>;

[[group(2), binding(1)]]
var s_historyTexture: sampler;

[[group(2), binding(2)]]
var t_outputTexture: texture_storage_2d<rgba16float, write>;

var<workgroup> s_color_pq : array<array<vec2<u32>, 19>, 19>;
var<workgroup> s_motion : array<array<u32, 19>, 19>;

let pq_m1 = 0.1593017578125;
let pq_m2 = 78.84375;
let pq_c1 = 0.8359375;
let pq_c2 = 18.8515625;
let pq_c3 = 18.6875;
let pq_C = 1.;//10000.0;

fn PQDecode(image: vec3<f32>) -> vec3<f32>
{
    let inv_pq_m2 = 1.0 / pq_m2;
    let inv_pq_m1 = 1.0 / pq_m1;
    let Np = pow(max(image, vec3<f32>(0.)), vec3<f32>(inv_pq_m2));
    var L = Np - pq_c1;
    L = L / (pq_c2 - pq_c3 * Np);
    L = pow(max(L, vec3<f32>(0.)), vec3<f32>(inv_pq_m1));

    return L * pq_C; // returns cd/m^2
}

fn PQEncode(image: vec3<f32>) -> vec3<f32>
{
    let L = image / pq_C;
    let Lm = pow(max(L, vec3<f32>(0.)), vec3<f32>(pq_m1));
    var N = (pq_c1 + pq_c2 * Lm) / (1.0 + pq_c3 * Lm);
    N = pow(N, vec3<f32>(pq_m2));

    return clamp(N, vec3<f32>(0.), vec3<f32>(1.));
}

// Preload the color data into shared memory, convert to PQ space
// Also preload the 2D motion vectors
fn preload(group_base: vec2<i32>, group_size: vec2<i32>, local_invocation_index: i32, input_size: vec2<i32>)
{
    let preload_size = min(group_size + 3, vec2<i32>(19));

    for(var linear_idx = local_invocation_index; linear_idx < preload_size.x * preload_size.y; linear_idx = linear_idx + 256)
    {
        // Convert the linear index to 2D index in a (preload_size x preload_size) virtual group
        let t = (f32(linear_idx) + 0.5) / f32(preload_size.x);
        let xx = i32(floor(fract(t) * f32(preload_size.x)));
        let yy = i32(floor(t));

        // Load
        var ipos = group_base + vec2<i32>(xx, yy) - 1;
        ipos = clamp(ipos, vec2<i32>(0, 0), vec2<i32>(input_size) - 1);
        let color = textureLoad(t_colorTexture, ipos, 0);
        let color_pq = PQEncode(color.rgb);
        let motion = vec2<f32>(0., 0.); // TODO

        // Store
        s_color_pq[yy][xx] = vec2<u32>(pack2x16float(color_pq.rg), pack2x16float(vec2<f32>(color_pq.b, color.a)));
        s_motion[yy][xx] = pack2x16float(motion);
    }
}

fn get_shared_color(pos: vec2<i32>, group_base: vec2<i32>) -> vec3<f32>
{
    let addr = pos - group_base + 1;
    
    let data = s_color_pq[addr.y][addr.x];
    return vec3<f32>(unpack2x16float(data.x), unpack2x16float(data.y).x);
}

fn get_shared_motion(pos: vec2<i32>, group_base: vec2<i32>) -> vec2<f32>
{
    let addr = pos - group_base + 1;
    
    return unpack2x16float(s_motion[addr.y][addr.x]);
}


struct Moments {
    mom1: vec3<f32>;
    mom2: vec3<f32>;
};

fn get_moments(pos: vec2<i32>, group_base: vec2<i32>, r: i32) -> Moments
{
    var mom: Moments;
    mom.mom1 = vec3<f32>(0.0);
    mom.mom2 = vec3<f32>(0.0);

    for(var yy = -r; yy <= r; yy = yy + 1)
    {
        for(var xx = -r; xx <= r; xx = xx + 1)
        {
            if(xx == 0 && yy == 0) {
                continue;
            }

            let p = pos + vec2<i32>(xx, yy);
            let c = get_shared_color(p, group_base);

            mom.mom1 = mom.mom1 + c.rgb;
            mom.mom2 = mom.mom2 + c.rgb * c.rgb;
        }
    }

    return mom;
}

fn get_sample_weight(delta: vec2<f32>, scale: f32) -> f32
{
    return clamp(1. - scale * dot(delta, delta), 0., 1.);
}

fn hires_to_lores(ipos: vec2<i32>, input_size: vec2<i32>, output_size: vec2<i32>) -> vec2<f32>
{
    return (vec2<f32>(ipos) + vec2<f32>(0.5, 0.5)) * (vec2<f32>(input_size) / vec2<f32>(output_size)) - vec2<f32>(0.5, 0.5) - uniforms.sub_pixel_jitter;
}

// Catmull-Rom filtering code from http://vec3.ca/bicubic-filtering-in-fewer-taps/
// uv is in pixel coordinates
fn sample_texture_catmull_rom(tex: texture_2d<f32>, sam: sampler, uv: vec2<f32> ) -> vec4<f32>
{
    var sum: vec4<f32> = vec4<f32>(0., 0., 0., 0.);

    let invTexSize = 1.0 / vec2<f32>(textureDimensions(tex));

    let tc = floor(uv - 0.5) + 0.5;
    let f = uv - tc;
    let f2 = f * f;
    let f3 = f2 * f;

    let w0 = f2 - 0.5 * (f3 + f);
    let w1 = 1.5 * f3 - 2.5 * f2 + 1.;
    let w3 = 0.5 * (f3 - f2);
    let w2 = 1. - w0 - w1 - w3;

    var sampleWeight: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
        w0,
        w1 + w2,
        w3,
    );

    var sampleLoc: array<vec2<f32>, 3> = array<vec2<f32>, 3>(
        (tc - 1.) * invTexSize,
        (tc + w2 / sampleWeight[1]) * invTexSize,
        (tc + 2.) * invTexSize,
    );

    for(var i = 0; i < 3; i = i + 1) {
        for(var j = 0; j < 3; j = j + 1) {
            let uv = vec2<f32>(sampleLoc[j].x, sampleLoc[i].y);
            let c = textureSampleLevel(tex, sam, uv, 0.);
            sum = sum + c * vec4<f32>(sampleWeight[j].x * sampleWeight[i].y);        
        }
    }
    return sum;
}

[[stage(compute), workgroup_size(16, 16)]]
fn main(
    [[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>,
    [[builtin(workgroup_id)]] workgroup_id: vec3<u32>, 
    [[builtin(local_invocation_index)]] local_invocation_index: u32
    ) {
    let input_size = textureDimensions(t_colorTexture);
    let output_size = textureDimensions(t_outputTexture);

    let group_base_hires = vec2<i32>(workgroup_id.xy) * 16;
    let group_base_lores = vec2<i32>(hires_to_lores(group_base_hires, input_size, output_size));
    let group_bottomright_hires = vec2<i32>(workgroup_id.xy) * 16 + 15;
    let group_bottomright_lores = vec2<i32>(hires_to_lores(group_bottomright_hires, input_size, output_size));

    preload(group_base_lores, group_bottomright_lores - group_base_lores + 1, i32(local_invocation_index), input_size.xy);
    workgroupBarrier();

    let ipos = vec2<i32>(global_invocation_id.xy);

    if (ipos.x >= output_size.x || ipos.y >= output_size.y)
    {
        return;
    }

    // Calculate position in the render buffer (at the lower render resolution)
    let nearest_render_pos = hires_to_lores(ipos, input_size, output_size);
    var int_render_pos = vec2<i32>(i32(round(nearest_render_pos.x)), i32(round(nearest_render_pos.y)));
    int_render_pos = clamp(int_render_pos, vec2<i32>(0, 0), vec2<i32>(i32(input_size.x) - 1, i32(input_size.y) - 1));

    var color_center = get_shared_color(int_render_pos, group_base_lores);
    
    var color_output: vec3<f32> = color_center;
    var linear_color_output: vec3<f32>;

    // Regular TAA/TAAU mode

    var mom: Moments;

    var num_pix: i32;

    // Obtain the color moments for the surrounding pixels.
    mom = get_moments(int_render_pos, group_base_lores, 1);
    num_pix = 9;
    
    // Remove or reduce sparkles by clamping the color of the center pixel to its surroundings
    if(flt_taa_anti_sparkle > 0.)
    {
        // Custom curve to make perceived blurriness depend on the cvar in a roughly linear way
        let scale = pow(min(1.0, flt_taa_anti_sparkle), -0.25);

        color_center = min(color_center, scale * mom.mom1 / f32(num_pix - 1));
    }

    mom.mom1 = (mom.mom1 + color_center) / f32(num_pix);
    mom.mom2 = (mom.mom2 + color_center * color_center) / f32(num_pix);

    // Find the longest motion vector in a 3x3 window
    var motion: vec2<f32>;
    {
        var len = -1.;
        let r = 1;
        for(var yy = -r; yy <= r; yy = yy + 1) {
            for(var xx = -r; xx <= r; xx = xx + 1) {
                let p = int_render_pos + vec2<i32>(xx, yy);
                let m = get_shared_motion(p, group_base_lores);
                let l = dot(m, m);
                if(l > len) {
                    len = l;
                    motion = m;
                }

            }
        }
    }

    let history_size = textureDimensions(t_historyTexture);

    // Calculate the previous position, taking into account that the previous frame output can have different size from the current frame
    let pos_prev = ((vec2<f32>(ipos) + vec2<f32>(0.5, 0.5)) / vec2<f32>(output_size) + motion.xy) * vec2<f32>(history_size);

    // Scale the motion for the weight calculation below
    motion = motion * vec2<f32>(output_size);

    if(all(vec2<i32>(pos_prev) >= vec2<i32>(1, 1))
        && all(vec2<i32>(pos_prev) < vec2<i32>(output_size) - 1))
    {
        // Motion vector was valid - sample the previous frame
        var color_prev = sample_texture_catmull_rom(t_historyTexture, s_historyTexture, pos_prev).rgb;

        //if(!any(isNan(color_prev)))
        {
            // If enabled, apply neighbourhood color clamping (NCC)
            if(flt_taa_variance > 0.)
            {
                let variance_scale = flt_taa_variance;

                let sigma = sqrt(max(vec3<f32>(0.), mom.mom2 - mom.mom1 * mom.mom1));
                let mi = mom.mom1 - sigma * variance_scale;
                let ma = mom.mom1 + sigma * variance_scale;

                color_prev = clamp(color_prev, mi, ma);
            }

            // Mix the new color with the clamped previous color
            let motion_weight = smoothStep(0., 1., sqrt(dot(motion, motion)));
            let sample_weight = get_sample_weight(nearest_render_pos - vec2<f32>(int_render_pos), f32(output_size.x) / f32(input_size.x));
            var pixel_weight = max(motion_weight, sample_weight) * 0.1;
            pixel_weight = clamp(pixel_weight, 0., 1.);
            color_output = mix(color_prev, color_center, pixel_weight);
        }
    }

    linear_color_output = PQDecode(color_output);
    textureStore(t_outputTexture, ipos, vec4<f32>(linear_color_output, 1.0));
}