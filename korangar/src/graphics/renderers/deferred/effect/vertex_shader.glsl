#version 450

layout(location = 0) out vec2 texture_coordinates;

layout(push_constant) uniform Constants {
    vec2 top_left;
    vec2 bottom_left;
    vec2 top_right;
    vec2 bottom_right;
    vec2 t_top_left;
    vec2 t_bottom_left;
    vec2 t_top_right;
    vec2 t_bottom_right;
    vec4 color;
} constants;

void main() {
    vec2 positions[6] = vec2[] (
        constants.top_left,
        constants.bottom_left,
        constants.top_right,
        constants.top_right,
        constants.bottom_left,
        constants.bottom_right
    );

    vec2 t_positions[6] = vec2[] (
        constants.t_top_left,
        constants.t_bottom_left,
        constants.t_top_right,
        constants.t_top_right,
        constants.t_bottom_left,
        constants.t_bottom_right
    );

    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
    texture_coordinates = t_positions[gl_VertexIndex];
}
