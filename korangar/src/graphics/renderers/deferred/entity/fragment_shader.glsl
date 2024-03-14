#version 450

layout(location = 0) in vec2 texture_coordinates;
layout(location = 1) in vec3 normal;
layout(location = 2) in float depth_offset;
layout(location = 3) in float curvature;

layout(location = 0) out vec4 fragment_color;
layout(location = 1) out vec3 fragment_normal;

layout(push_constant) uniform Constants {
    mat4 world;
    vec2 texture_position;
    vec2 texture_size;
    float depth_offset;
    float curvature;
    bool mirror;
} constants;

layout (set = 1, binding = 0) uniform sampler2D sampled_texture;

void main() {

    vec4 diffuse_color = texture(sampled_texture, texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    fragment_color = diffuse_color;
    fragment_normal = normalize(normal);

    float curvature_offset = (0.5 - pow(curvature, 2)) * constants.curvature;
    gl_FragDepth = gl_FragCoord.z + depth_offset + curvature_offset;
}
