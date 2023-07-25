#version 450

layout(location = 0) in vec2 texture_coordinates;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec4 fragment_color;
layout(location = 1) out vec3 fragment_normal;

layout(set = 0, binding = 0) uniform sampler2D sampled_texture;

layout(push_constant) uniform Constants {
    mat4 view_projection;
    vec3 color;
    vec3 upper_left;
    vec3 upper_right;
    vec3 lower_left;
    vec3 lower_right;
} constants;

void main() {
    fragment_color = texture(sampled_texture, texture_coordinates);

    if (fragment_color.a < 0.1) {
        discard;
    }

    fragment_color.rgb *= constants.color;
    fragment_normal = normal;
}
