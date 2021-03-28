#version 450

layout(location = 0)in vec2 v_tex_coords;

layout(set = 0, binding = 0)uniform texture2D t_color_attachment;
layout(set = 0, binding = 1)uniform texture2D t_tonemap;
layout(set = 0, binding = 2)uniform sampler s_color_sampler;

layout(set = 1, binding = 0)uniform LcdUniforms {
    vec2 u_viewport_size;
    vec2 u_pixels_per_unit;
};

layout(location = 0)out vec4 f_color;

// ---------------------------------------------------------------------------------------------------------------------

#define GRID_ALPHA 0.5
#define GRID_HARDNESS 1.75
#define SHADOW_ALPHA .75


float soft_grid(vec2 st, vec2 viewport_size, vec2 pixels_per_unit) {
    float aspect = viewport_size.x / viewport_size.y;
    vec2 coord = (st * pixels_per_unit * viewport_size);
    vec2 offset = fract(aspect) / pixels_per_unit;
    coord.x -= offset.x;
    coord.y += offset.y;
    vec2 dist = abs(fract(coord) - 0.5) * 2.;
    dist *= dist;
    dist = pow(dist, vec2(GRID_HARDNESS));

    float i = min(dist.r + dist.g, 1.0);
    return i;
}

float inner_shadow(vec2 st, float x_width, float y_width) {
    float left = max(1.0 - (st.x / x_width), 0.0);
    float right = 1.0 - min((1.0 - st.x) / x_width, 1.0);
    float top = 1.0 - min((1.0 - st.y) / y_width, 1.0);

    left = pow(left, 4.0);
    right = pow(right, 4.0);
    top = pow(top, 4.0);

    return min(left + right + top, 1.0);
}

void main() {
    vec2 tex_coord = vec2(v_tex_coords.s, 1 - v_tex_coords.t);
    float intensity = texture(sampler2D(t_color_attachment, s_color_sampler), tex_coord).r;

    // apply tonemap (note: tonemap has 4 entries, so we offset halfway into the
    // map by adding 0.25 * 0.5 - this stabilizes the tonemap output)
    vec4 palettized_color =
    texture(sampler2D(t_tonemap, s_color_sampler), vec2(intensity + 0.125, 0));

    // get the "white" value for our tonemap
    vec4 grid_color = texture(sampler2D(t_tonemap, s_color_sampler), vec2(1.0, 1.0));
    float grid = soft_grid(v_tex_coords, u_viewport_size, u_pixels_per_unit);
    palettized_color.rgb = mix(palettized_color.rgb, grid_color.rgb, grid * GRID_ALPHA);

    vec3 shadow_color = vec3(0.0);
    palettized_color.rgb = mix(palettized_color.rgb, shadow_color.rgb, SHADOW_ALPHA * inner_shadow(v_tex_coords, 0.1, 0.2));

    f_color = vec4(palettized_color.rgb, 1.0);
}