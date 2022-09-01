#version 450

layout(location = 0) in vec3 position;
layout(location = 2) in vec2 texture_coordinates;

layout(location = 0) out vec2 texture_coordinates_out;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 view;
    mat4 projection;
} matrices;

layout(push_constant) uniform Constants {
    mat4 world;
    vec2 texture_position;
    vec2 texture_size;
    bool mirror;
} constants;

void main() {
    gl_Position = matrices.projection * matrices.view * constants.world * vec4(position, 1.0);
    texture_coordinates_out = constants.texture_position + texture_coordinates * constants.texture_size;

    if (constants.mirror) {
        texture_coordinates_out.x = 1 - texture_coordinates_out.x;
    }
}
