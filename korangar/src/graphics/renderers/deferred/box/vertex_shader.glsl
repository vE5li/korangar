#version 450

layout(location = 0) in vec3 position;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 view_projection;
} matrices;

layout(push_constant) uniform Constants {
    mat4 world;
    vec3 color;
} constants;

void main() {
    gl_Position = matrices.view_projection * constants.world * vec4(position, 1.0);
}
