#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 texture_coordinates;

layout(location = 0) out vec2 texture_coordinates_out;
layout(location = 1) out vec3 normal_out;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 world;
    mat4 view;
    mat4 projection;
} uniforms;

void main() {
    gl_Position = uniforms.projection * uniforms.view * uniforms.world * vec4(position, 1.0);
    texture_coordinates_out = texture_coordinates;
    normal_out = vec4(uniforms.world * vec4(normal, 1.0)).xyz;
}
