#version 450

layout(location = 0) in vec2 texture_coordinates;
layout(location = 1) in float depth_offset;
layout(location = 2) in float curvature;

layout (set = 1, binding = 0) uniform sampler2D sampled_texture;

layout(push_constant) uniform Constants {
    mat4 world;
    vec2 texture_position;
    vec2 texture_size;
    float depth_offset;
    float curvature;
    bool mirror;
} constants;

void main() {
    vec4 diffuse_color = texture(sampled_texture, texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    // We use min here to only make shadow entities curve back. This helps reduce self-shadowing
    float curvature_offset = min(0, (0.5 - pow(curvature, 2)) * constants.curvature);
    gl_FragDepth = gl_FragCoord.z + depth_offset - curvature_offset;
}
