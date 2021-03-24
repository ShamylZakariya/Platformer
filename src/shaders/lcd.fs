#version 450

layout(location = 0) in vec2 v_tex_coords;

layout(set = 0, binding = 0) uniform texture2D color_attachment;
layout(set = 0, binding = 1) uniform sampler color_sampler;

layout(location = 0) out vec4 f_color;

void main() {
    vec2 tex_coord = vec2(v_tex_coords.s, 1 - v_tex_coords.t);
    vec3 color_attachment_value = texture(sampler2D(color_attachment, color_sampler), tex_coord).xyz;
    color_attachment_value *= vec3(v_tex_coords.s, v_tex_coords.t, 1);
    f_color = vec4(color_attachment_value, 1.0);
}