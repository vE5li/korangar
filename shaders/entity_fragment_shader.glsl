#version 450

const int TEXTURE_COUNT = 10;

layout(location = 0) in vec2 texture_coordinates;

layout(location = 0) out vec4 fragment_color;
layout(location = 1) out vec3 fragment_normal;

layout (set = 0, binding = 1) uniform sampler2D sampled_texture;

void main() {

    vec4 diffuse_color = texture(sampled_texture, texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    fragment_color = diffuse_color;
    fragment_normal = vec3(0.0, 0.0, 0.0);
}
