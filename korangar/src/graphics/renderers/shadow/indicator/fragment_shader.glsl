#version 450

layout(location = 0) in vec2 texture_coordinates;

layout(set = 0, binding = 0) uniform sampler2D sampled_texture;

layout(push_constant) uniform Constants {
    mat4 view_projection;
    vec3 upper_left;
    vec3 upper_right;
    vec3 lower_left;
    vec3 lower_right;
} constants;

void main() {
    vec4 fragment_color = texture(sampled_texture, texture_coordinates);

    if (fragment_color.a < 0.1) {
        discard;
    }
}
