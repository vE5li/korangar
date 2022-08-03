#version 450

layout(location = 0) out vec4 fragment_color;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    vec4 color;
} constants;

void main() {
    fragment_color = constants.color;
}
