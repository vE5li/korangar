#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec2 texture_coordinates;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    vec2 texture_position;
    vec2 texture_size;
    vec3 color;
} constants;

void main() {
    vec2 vertex_position = position * constants.screen_size - vec2(1.0) + constants.screen_position;
    gl_Position = vec4(vertex_position, 0.0, 1.0);

    texture_coordinates = position * constants.texture_size + constants.texture_position;
}
