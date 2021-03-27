#version 450

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_tex_coords;
layout(location = 2) in vec2 a_corner;
layout(location = 3) in vec4 a_color;

layout(location = 0) out vec2 v_tex_coords;
layout(location = 1) out vec4 v_color;

layout(set = 1, binding = 0) uniform CameraUniforms {
  vec3 u_position;         // camera aposition world
  mat4 u_view_proj;        // camera view * proj
  vec2 u_framebuffer_size; // pixel size of framebuffer
};

layout(set = 2, binding = 0) uniform SpriteUniforms {
  vec4 u_model_position;
  vec4 u_color;
  vec2 u_sprite_scale;
  vec2 u_pixels_per_unit;
  vec2 u_tex_coord_offset;
  float u_palette_shift;
};

void main() {
  v_tex_coords = a_tex_coords + u_tex_coord_offset;
  v_color = a_color * u_color;

  vec2 position = (u_sprite_scale * a_position.xy) + u_model_position.xy;
  position = round(position * u_pixels_per_unit) / u_pixels_per_unit;

  // compute half-pixel outset bleed to mitigate cracking, since we can't use
  // indexed meshes because of non-continous tex coord assignment
  vec2 outset = vec2(0.25 / u_framebuffer_size.x, 0.25 / u_framebuffer_size.y);
  position += a_corner * outset;

  gl_Position = u_view_proj * vec4(position.x, position.y,
                                   a_position.z + u_model_position.z, 1.0);
}