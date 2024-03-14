#version 450

layout(location = 0) in vec2 texture_coordinates;

layout(location = 0) out vec4 fragment_color;

layout (set = 0, binding = 0) uniform sampler2D sprite_texture;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    vec2 texture_position;
    vec2 texture_size;
    vec4 color;
} constants;

void main() {
    fragment_color = texture(sprite_texture, texture_coordinates) * constants.color;
}
