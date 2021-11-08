[[block]]
struct Uniforms {
    translate: vec2<f32>;
    view_size: vec2<f32>;
    world_size: vec2<f32>;
    inv_world_size: vec2<f32>;
    pixel_size: vec2<f32>;
    mouse: vec2<f32>;
    cursor_size: f32;
    time: f32;
    exposure: f32;
};

[[group(0), binding(0)]]
var uniforms: Uniforms;

[[group(1), binding(0)]]
var t_colorTexture: texture_2d<f32>;

[[group(2), binding(0)]]
var t_historyTexture: texture_2d<f32>;

[[group(2), binding(1)]]
var s_historyTexture: sampler;

[[group(2), binding(2)]]
var t_outputTexture: texture_storage_2d<rgba16float, write>;

fn get_sample_weight(delta: vec2<f32>, scale: f32) -> f32
{
    return clamp(1. - scale * dot(delta, delta), 0., 1.);
}

fn hires_to_lores(ipos: vec2<i32>) -> vec2<f32>
{
    let input_size = vec2<f32>(textureDimensions(t_colorTexture));
    let output_size = vec2<f32>(textureDimensions(t_outputTexture));

    return (vec2<f32>(ipos) + vec2<f32>(0.5, 0.5)) * (input_size / output_size) - vec2<f32>(0.5, 0.5);// - global_ubo.sub_pixel_jitter;
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
fn main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>) {
    let ipos = vec2<i32>(global_invocation_id.xy);

    let output_size = textureDimensions(t_outputTexture);

    if (ipos.x >= output_size.x || ipos.y >= output_size.y)
    {
        return;
    }

    let input_size = textureDimensions(t_colorTexture);
    let history_size = textureDimensions(t_historyTexture);
    let motion = vec2<f32>(0., 0.); // TODO

    // Calculate position in the render buffer (at the lower render resolution)
    let nearest_render_pos = hires_to_lores(ipos);
    var int_render_pos = vec2<i32>(i32(round(nearest_render_pos.x)), i32(round(nearest_render_pos.y)));
    int_render_pos = clamp(int_render_pos, vec2<i32>(0, 0), vec2<i32>(i32(input_size.x) - 1, i32(input_size.y) - 1));

    let color_center = textureLoad(t_colorTexture, int_render_pos, 0).rgb;
    
    // Calculate the previous position, taking into account that the previous frame output can have different size from the current frame
    let pos_prev = ((vec2<f32>(ipos) + vec2<f32>(0.5, 0.5)) / vec2<f32>(output_size) + motion.xy) * vec2<f32>(history_size);

    var color_output: vec3<f32> = color_center;

    if(all(vec2<i32>(pos_prev) >= vec2<i32>(1, 1))
        && all(vec2<i32>(pos_prev) < vec2<i32>(output_size) - 1))
    {
        // Motion vector was valid - sample the previous frame
        let color_prev = sample_texture_catmull_rom(t_historyTexture, s_historyTexture, pos_prev).rgb;

        //if(!any(isNan(color_prev)))
        //{
            // Mix the new color with the clamped previous color
            let motion_weight = smoothStep(0., 1., sqrt(dot(motion, motion)));
            let sample_weight = get_sample_weight(nearest_render_pos - vec2<f32>(int_render_pos), f32(output_size.x) / f32(input_size.x));
            var pixel_weight = max(motion_weight, sample_weight) * 0.1f;
            pixel_weight = clamp(pixel_weight, 0., 1.);
            color_output = mix(color_prev, color_center, pixel_weight);
        //}
    }

    textureStore(t_outputTexture, ipos, vec4<f32>(color_output, 1.0));
}