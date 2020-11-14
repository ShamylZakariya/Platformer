#version 450

layout(location = 0) in vec2 v_tex_coords;
layout(location = 1) in vec4 v_color;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform texture2D t_diffuse;
layout(set = 0, binding = 1) uniform sampler s_diffuse;

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
  vec4 object_color =
      v_color * texture(sampler2D(t_diffuse, s_diffuse), v_tex_coords);
  f_color = object_color;
}