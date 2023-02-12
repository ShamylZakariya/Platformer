struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

struct LcdUniforms {
    camera_position: vec2<f32>,
    viewport_size: vec2<f32>,
    pixels_per_unit: vec2<f32>,
    pixel_effect_alpha: f32,
    shadow_effect_alpha: f32,
    color_attachment_layer_index: u32,
    color_attachment_layer_count: u32,
};

@group(0) @binding(0)
var color_attachment_texture: texture_2d_array<f32>;

@group(0) @binding(1)
var tonemap_texture: texture_2d<f32>;

@group(0) @binding(2)
var color_sampler: sampler;

@group(1) @binding(0)
var<uniform> lcd_uniforms: LcdUniforms;

///////////////////////////////////////////////////////////////////////

let PIXEL_EFFECT_ALPHA:f32 = 0.5;
let PIXEL_EFFECT_HARDNESS:f32 = 2.75;
let SHADOW_ALPHA:f32 = 0.75;

fn soft_grid(st: vec2<f32>, camera_position: vec2<f32>, viewport_size: vec2<f32>, pixels_per_unit: vec2<f32>) -> f32 {
    // camera is centered, so we count pixels out from center
    let coord = ((st - vec2(0.5)) * pixels_per_unit * viewport_size);
    let dist = abs(fract(coord) - 0.5) * 2.0;
    let dist = pow(dist, vec2(PIXEL_EFFECT_HARDNESS));

    let i = min(dist.r + dist.g, 1.0);
    return i;
}

fn inner_shadow(st: vec2<f32>, x_width: f32, y_width: f32) -> f32 {
    let left = max(1.0 - (st.x / x_width), 0.0);
    let right = 1.0 - min((1.0 - st.x) / x_width, 1.0);
    let top = 1.0 - min((1.0 - st.y) / y_width, 1.0);

    let left = pow(left, 4.0);
    let right = pow(right, 4.0);
    let top = pow(top, 4.0);

    return min(left + right + top, 1.0);
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

    let intensity = textureSample(color_attachment_texture, color_sampler, in.tex_coord, 0).r;

    // apply tonemap (note: tonemap has 4 entries, so we offset halfway into the
    // map by adding 0.25 * 0.5 - this stabilizes the tonemap output)
    let palettized_color = textureSample(tonemap_texture, color_sampler, vec2<f32>(intensity + 0.125, 0.0));

    // get the "white" value for our tonemap, and the pixel effect amount. Mix in
    // the pixel effect amount by lcd_uniforms.pixel_effect_alpha which goes to zero as the
    // user window size changes, to reduce moire` effects. But in turn, mix in the
    // raw grid_color inversely to prevent an overall darkening of the scene.
    let grid_color = textureSample(tonemap_texture, color_sampler, vec2<f32>(1.0, 1.0));

    let grid = soft_grid(in.tex_coord, lcd_uniforms.camera_position, lcd_uniforms.viewport_size, lcd_uniforms.pixels_per_unit);

    let palettized_color = mix(palettized_color.xyz, grid_color.xyz, grid * PIXEL_EFFECT_ALPHA * lcd_uniforms.pixel_effect_alpha);

    let palettized_color = mix(palettized_color, grid_color.xyz, 0.5 * PIXEL_EFFECT_ALPHA * (1.0 - lcd_uniforms.pixel_effect_alpha));

    let shadow_color = vec3<f32>(0.0);
    let palettized_color = mix(palettized_color, shadow_color, SHADOW_ALPHA * lcd_uniforms.shadow_effect_alpha * inner_shadow(in.tex_coord, 0.1, 0.2));

    return vec4<f32>(palettized_color, 1.0);
}