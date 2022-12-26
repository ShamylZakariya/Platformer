struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) corner: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct CameraUniforms {
    position: vec4<f32>,
    view_proj: mat4x4<f32>,
    framebuffer_size: vec4<f32>,
};

struct SpriteUniforms {
    model_position: vec4<f32>,
    color: vec4<f32>,
    sprite_scale: vec2<f32>,
    pixels_per_unit: vec2<f32>,
    tex_coord_offset: vec2<f32>,
    palette_shift: f32,
    unused_: f32,
};

@group(0) @binding(0)
var sprite_texture: texture_2d<f32>;

@group(0) @binding(1)
var sprite_sampler: sampler;

@group(1) @binding(0)
var<uniform> camera_uniforms: CameraUniforms;

@group(2) @binding(0)
var<uniform> sprite_uniforms: SpriteUniforms;

///////////////////////////////////////////////////////////////////////

@vertex
fn sprite_vs_main(in: VertexInput) -> FragmentInput {

    let position = (sprite_uniforms.sprite_scale * in.position.xy) + sprite_uniforms.model_position.xy;
    let position = round(position * sprite_uniforms.pixels_per_unit) / sprite_uniforms.pixels_per_unit;

    // compute half-pixel outset bleed to mitigate cracking, since we can't use
    // indexed meshes because of non-continous tex coord assignment

    let outset = vec2<f32>(0.25 / camera_uniforms.framebuffer_size.x, 0.25 / camera_uniforms.framebuffer_size.y);
    let position = position + in.corner * outset;

    var out: FragmentInput;
    out.tex_coords = in.tex_coords + sprite_uniforms.tex_coord_offset;
    out.color = in.color * sprite_uniforms.color;
    out.clip_position = camera_uniforms.view_proj * vec4<f32>(position.x, position.y, in.position.z + sprite_uniforms.model_position.z, 1.0);

    return out;
}

@fragment
fn sprite_fs_main(in: FragmentInput) -> @location(0) vec4<f32> {

    var object_color = in.color * textureSample(sprite_texture, sprite_sampler, in.tex_coords);
    if object_color.a == 0.0 {
        discard;
    }

    // treat palette shift as color scale
    if sprite_uniforms.palette_shift > 0.0 {
        object_color = mix(object_color, vec4<f32>(1.0, 1.0, 1.0, object_color.a), sprite_uniforms.palette_shift);
    } else {
        object_color = mix(object_color, vec4<f32>(0.0, 0.0, 0.0, object_color.a), -sprite_uniforms.palette_shift);
    }

    return object_color;
}