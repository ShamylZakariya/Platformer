#version 450

layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_tex_coords;
layout(location = 2) in vec4 a_color;

layout(location = 0) out vec2 v_tex_coords;
layout(location = 1) out vec4 v_color;

layout(set = 1, binding = 0) uniform CameraUniforms {
  vec3 u_view_position;
  mat4 u_view_proj;
};

layout(set = 2, binding = 0) uniform SpriteUniforms {
  vec4 u_model_position;
  vec4 u_color;
  vec2 u_sprite_size_px;
};

void main() {
  v_tex_coords = a_tex_coords;
  v_color = a_color * u_color;

  vec2 position = a_position.xy + u_model_position.xy;
  position = round(position * u_sprite_size_px) / u_sprite_size_px;

  gl_Position = u_view_proj * vec4(position.x, position.y,
                                   a_position.z + u_model_position.z, 1.0);
}