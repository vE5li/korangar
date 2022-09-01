#version 450

layout(location = 0) out uint fragment_color;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    uint identifier;
} constants;

void main() {
    fragment_color = constants.identifier;
}
