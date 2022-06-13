#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec2 position_out;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    vec2 clip_size;
    vec4 corner_radius;
    vec3 color;
} constants;

void main() {
    vec2 vertex_position = constants.screen_position - vec2(1.0) + position * constants.screen_size;
    gl_Position = vec4(vertex_position, 0.0, 1.0);
    position_out = position;
}
