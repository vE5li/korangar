#version 450

layout(location = 0) in vec2 texture_coordinates;

layout(location = 0) out uint fragment_color;

layout (set = 1, binding = 0) uniform sampler2D sampled_texture;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 view;
    mat4 projection;
} matrices;

layout(push_constant) uniform Constants {
    mat4 world;
    vec2 texture_position;
    vec2 texture_size;
    uint identifier;
    bool mirror;
} constants;

void main() {

    vec4 diffuse_color = texture(sampled_texture, texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    fragment_color = constants.identifier;
}
