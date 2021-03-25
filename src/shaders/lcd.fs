#version 450

layout(location = 0) in vec2 v_tex_coords;

layout(set = 0, binding = 0) uniform texture2D color_attachment;
layout(set = 0, binding = 1) uniform texture2D tonemap;
layout(set = 0, binding = 2) uniform sampler color_sampler;

layout(set = 1, binding = 0) uniform LcdUniforms {
  vec2 u_sprite_size_px;
  float u_palette_shift;
};

layout(location = 0) out vec4 f_color;

void main() {
  vec2 tex_coord = vec2(v_tex_coords.s, 1 - v_tex_coords.t);
  float intensity = texture(sampler2D(color_attachment, color_sampler), tex_coord).r;

  // apply tonemap (note: tonemap has 4 entries, so we offset halfway into the
  // map by adding 0.25 * 0.5 - this stabilizes the tonemap output)
  vec4 palettized_color =
      texture(sampler2D(tonemap, color_sampler), vec2(intensity + 0.125, 0));

  f_color = vec4(palettized_color.rgb, 1.0);
}