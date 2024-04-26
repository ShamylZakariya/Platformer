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
var color_sampler: sampler;

@group(1) @binding(0)
var<uniform> lcd_uniforms: LcdUniforms;

///////////////////////////////////////////////////////////////////////////////

@vertex
fn lcd_column_average_vs_main(@builtin(vertex_index) in_vertex_index: u32) -> FragmentInput {
    // wgsl doesn't let us index `let` arrays with a variable. So it has to be a `var` local to this function.
    var fsq_clip_positions: array<vec4<f32>,3> = array<vec4<f32>, 3>(vec4<f32>(-1.0, 1.0, 0.0, 1.0), vec4<f32>(3.0, 1.0, 0.0, 1.0), vec4<f32>(-1.0, -3.0, 0.0, 1.0));
    var fsq_tex_coords: array<vec2<f32>,3> = array<vec2<f32>, 3>(vec2<f32>(0.0, 0.0), vec2<f32>(2.0, 0.0), vec2<f32>(0.0, 2.0));

    var out: FragmentInput;
    out.tex_coord = fsq_tex_coords[in_vertex_index];
    out.clip_position = fsq_clip_positions[in_vertex_index];

    return out;
}

fn average_column(tc: vec2<f32>) -> f32 {
    let rows:u32 = color_attachment_size.y;
    let row_step: f32 = 1.0 / f32(rows);
    let layer = lcd_uniforms.color_attachment_layer_index;

    var accumulator:f32 = 0.0;
    var row_tc = 0.0;

    for (var i:u32 = 0; i < rows; i += 1_u32) {
        let tex_coord = vec2<f32>(tc.x, row_tc);
        accumulator += textureSample(color_attachment_texture, color_sampler, tex_coord, layer).r;
        row_tc += row_step;
    }

    return accumulator / f32(rows);
}

@fragment
fn lcd_column_average_fs_main_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let v = average_column(in.tc);
    return vec4<f32>(v, v, v, 1.0);
}