#version 450

layout(location = 0) in vec3 position;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 view;
    mat4 projection;
} matrices;

layout(push_constant) uniform Constants {
    mat4 world;
} constants;

void main() {
    gl_Position = matrices.projection * matrices.view * constants.world * vec4(position, 1.0);
}
