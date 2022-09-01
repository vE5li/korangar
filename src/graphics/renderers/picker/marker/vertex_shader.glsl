#version 450

layout(location = 0) in vec2 position;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    uint identifier;
} constants;

void main() {
    vec2 vertex_position = constants.screen_position - vec2(1.0) + position * constants.screen_size;
    gl_Position = vec4(vertex_position, 0.0, 1.0);
}
