#version 450

layout(location = 0) in vec3 position;
layout(location = 3) in uint identifier;

layout(location = 0) out flat uint identifier_out;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 view_projection;
} matrices;

void main() {
    gl_Position = matrices.view_projection * vec4(position, 1.0);
    identifier_out = identifier;
}
