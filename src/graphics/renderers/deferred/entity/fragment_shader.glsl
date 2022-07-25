#version 450

layout(location = 0) in vec2 texture_coordinates;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec4 fragment_color;
layout(location = 1) out vec3 fragment_normal;

layout (set = 1, binding = 0) uniform sampler2D sampled_texture;

void main() {

    vec4 diffuse_color = texture(sampled_texture, texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    fragment_color = diffuse_color;

    fragment_color.r = pow(fragment_color.r, 1.0 / 1.8);
    fragment_color.g = pow(fragment_color.g, 1.0 / 1.8);
    fragment_color.b = pow(fragment_color.b, 1.0 / 1.8);

    fragment_normal = normalize(normal);
}
