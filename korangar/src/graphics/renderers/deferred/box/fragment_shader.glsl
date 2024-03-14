#version 450

layout(location = 0) out vec4 fragment_color;

layout(push_constant) uniform Constants {
    mat4 world;
    vec3 color;
} constants;

void main() {
    fragment_color = vec4(constants.color, 1.0);
}
