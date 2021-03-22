#version 450

layout(location = 0) in vec2 v_tex_coords;

layout(location = 0) out vec4 f_color;

void main() {
    f_color = vec4(v_tex_coords.s, v_tex_coords.y, 0.0, 1.0);
}