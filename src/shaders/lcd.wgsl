struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

struct LcdUniforms {
    camera_position: vec2<f32>,
    viewport_size: vec2<f32>,
    pixels_per_unit: vec2<f32>,
    lcd_resolution: vec2<f32>,
    pixel_effect_alpha: f32,
    shadow_effect_alpha: f32,
    color_attachment_size: vec2<u32>,
    color_attachment_layer_index: u32,
    color_attachment_layer_count: u32,
    color_attachment_history_count: u32,
    padding_: u32,
};

@group(0) @binding(0)
var color_attachment_texture: texture_2d_array<f32>;

@group(0) @binding(1)
var tonemap_texture: texture_2d<f32>;

@group(0) @binding(2)
var column_average_weights_texture: texture_2d<f32>;

@group(0) @binding(3)
var color_sampler: sampler;

@group(1) @binding(0)
var<uniform> lcd_uniforms: LcdUniforms;

///////////////////////////////////////////////////////////////////////

fn rand2(n: vec2<f32>) -> f32 {
    return fract(sin(dot(n, vec2<f32>(12.9898, 4.1414))) * 43758.5453);
}

fn noise2(p: vec2<f32>) -> f32 {
    let ip = floor(p);
	var u = fract(p);
	u = u*u*(3.0-(2.0*u));

	let res = mix(
		mix(rand2(ip),rand2(ip+vec2(1.0,0.0)),u.x),
		mix(rand2(ip+vec2(0.0,1.0)),rand2(ip+vec2(1.0,1.0)),u.x),u.y);
	return res*res;
}

fn fbm(t: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var x = t;
    var shift = vec2<f32>(100.0, 100.0);
    var rot = mat2x2<f32>(cos(0.5), sin(0.5), -sin(0.5), cos(0.50));

    for (var i = 0u; i < 4u; i += 1u) {
        v += a * noise2(x);
        x = rot * (x * 2.0) + shift;
        a *= 0.5;
    }
    return v;
}

///////////////////////////////////////////////////////////////////////

const PIXEL_EFFECT_ALPHA:f32 = 0.65;
const PIXEL_EFFECT_HARDNESS:f32 = 3.0;
const REFLECTOR_SPARKLE:f32 = 0.1;

fn soft_grid(st: vec2<f32>, camera_position: vec2<f32>, viewport_size: vec2<f32>, pixels_per_unit: vec2<f32>) -> f32 {
    // camera is centered, so we count pixels out from center
    let coord = ((st - vec2(0.5)) * pixels_per_unit * viewport_size);
    var dist = abs(fract(coord) - 0.5) * 2.0;
    dist = pow(dist, vec2(PIXEL_EFFECT_HARDNESS));

    let i = min(dist.r + dist.g, 1.0);
    return i;
}

fn inner_shadow(st: vec2<f32>) -> f32 {
    let x_width = 0.2;
    let y_width = 0.4;
    let hardness = 3.0;
    let lumpiness_frequency = 10.0;
    let lumpiness_mix = 0.25;

    var left = max(1.0 - (st.x / x_width), 0.0);
    var right = 1.0 - min((1.0 - st.x) / x_width, 1.0);
    var top = 1.0 - min((1.0 - st.y) / y_width, 1.0);

    left = pow(left, hardness);
    right = pow(right, hardness);
    top = pow(top, hardness);

    let total = min(left + right + top, 1.0);
    var lumpiness = 1.0 - (lumpiness_mix * fbm(st * lumpiness_frequency));

    return lcd_uniforms.shadow_effect_alpha * total * lumpiness;
}

// returns a value from [-1,1]
fn lcd_reflector_sparkle(st: vec2<f32>) -> f32 {
    let n = rand2(st) * 2.0 - 1.0;
    return REFLECTOR_SPARKLE * n;
}

// returns the palettized sampled lcd color in (rgb), and the raw intensity in (alpha)
fn sample_palettized(tex_coord: vec2<f32>) -> vec4<f32> {
    let history_count = i32(lcd_uniforms.color_attachment_history_count);
    let layer_count = i32(lcd_uniforms.color_attachment_layer_count);
    let first_layer = (i32(lcd_uniforms.color_attachment_layer_index) + layer_count - (history_count - 1)) % layer_count;

    var accumulator = vec4<f32>(0.0);
    for (var i: i32 = 0; i < history_count; i++) {
        let layer = (first_layer + i) % layer_count;
        let intensity = textureSample(color_attachment_texture, color_sampler, tex_coord, layer).r;

        // apply tonemap (note: tonemap has 4 entries, so we offset halfway into the
        // map by adding 0.25 * 0.5 - this stabilizes the tonemap output)

        let palettized_color = textureSample(tonemap_texture, color_sampler, vec2<f32>(intensity + 0.125, 0.0));
        accumulator += vec4<f32>(palettized_color.xyz, intensity);
    }

    let averaged_color = accumulator / f32(history_count);
    let column_weight = textureSample(column_average_weights_texture, color_sampler, vec2<f32>(tex_coord.x, 0.0)).r;
    let column_bleed = pow(column_weight, 0.125);

    return averaged_color * column_bleed;
}

///////////////////////////////////////////////////////////////////////

@vertex
fn lcd_vs_main(@builtin(vertex_index) in_vertex_index: u32) -> FragmentInput {
    // wgsl doesn't let us index `let` arrays with a variable. So it has to be a `var` local to this function.
    var fsq_clip_positions: array<vec4<f32>,3> = array<vec4<f32>, 3>(vec4<f32>(-1.0, 1.0, 0.0, 1.0), vec4<f32>(3.0, 1.0, 0.0, 1.0), vec4<f32>(-1.0, -3.0, 0.0, 1.0));
    var fsq_tex_coords: array<vec2<f32>,3> = array<vec2<f32>, 3>(vec2<f32>(0.0, 0.0), vec2<f32>(2.0, 0.0), vec2<f32>(0.0, 2.0));

    var out: FragmentInput;
    out.tex_coord = fsq_tex_coords[in_vertex_index];
    out.clip_position = fsq_clip_positions[in_vertex_index];

    return out;
}

@fragment
fn lcd_fs_main(in: FragmentInput) -> @location(0) vec4<f32> {

    // get source color value. this will include slow-response lcd history if enabled;
    let lcd_sampled_value = sample_palettized(in.tex_coord);
    var lcd_pixel_color = lcd_sampled_value.xyz;
    let lcd_intensity = lcd_sampled_value.a;

    // get the "white" value for our tonemap, and the pixel effect amount. Mix in
    // the pixel effect amount by lcd_uniforms.pixel_effect_alpha which goes to zero as the
    // user window size changes, to reduce moire` effects. But in turn, mix in the
    // raw grid_color inversely to prevent an overall darkening of the scene.

    let grid_color = textureSample(tonemap_texture, color_sampler, vec2<f32>(1.0, 1.0));
    let grid = soft_grid(in.tex_coord, lcd_uniforms.camera_position, lcd_uniforms.viewport_size, lcd_uniforms.pixels_per_unit);

    lcd_pixel_color = mix(lcd_pixel_color, grid_color.xyz, grid * PIXEL_EFFECT_ALPHA * lcd_uniforms.pixel_effect_alpha);
    lcd_pixel_color = mix(lcd_pixel_color, grid_color.xyz, 0.5 * PIXEL_EFFECT_ALPHA * (1.0 - lcd_uniforms.pixel_effect_alpha));

    // mix in lcd back reflector "sparkle" based on opacity of the lcd cell,
    // e.g., the darker the pixel, the less sparkle
    var sparkle = lcd_reflector_sparkle(in.tex_coord);
    lcd_pixel_color += lcd_intensity * sparkle;

    // bake in the inner shadow
    let received_light = 1.0 - inner_shadow(in.tex_coord);
    lcd_pixel_color *= received_light;

    return vec4<f32>(lcd_pixel_color, 1.0);
}
