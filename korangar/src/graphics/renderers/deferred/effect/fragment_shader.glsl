#version 450

layout(location = 0) in vec2 texture_coordinates;

layout(location = 0) out vec4 fragment_color;

layout (set = 0, binding = 0) uniform sampler2D sprite_texture;

layout(push_constant) uniform Constants {
    vec2 top_left;
    vec2 bottom_left;
    vec2 top_right;
    vec2 bottom_right;
    vec2 t_top_left;
    vec2 t_bottom_left;
    vec2 t_top_right;
    vec2 t_bottom_right;
    vec4 color;
} constants;

void main() {
    fragment_color = texture(sprite_texture, texture_coordinates) * constants.color;
}
