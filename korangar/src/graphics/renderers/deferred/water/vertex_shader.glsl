#version 450

layout(location = 0) in vec3 position;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 view;
    mat4 projection;
} matrices;

layout(push_constant) uniform Constants {
    float wave_offset;
} constants;

void main() {
    vec3 adjusted_position = vec3(position.x, position.y + sin(constants.wave_offset + position.x + position.z), position.z);
    gl_Position = matrices.projection * matrices.view * vec4(adjusted_position, 1.0);
}
