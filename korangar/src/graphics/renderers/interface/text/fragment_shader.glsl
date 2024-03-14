#version 450

layout(location = 0) in vec2 texture_coordinates;

layout(location = 0) out vec4 fragment_color;

layout (set = 0, binding = 0) uniform sampler2D sprite_texture;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    vec4 screen_clip;
    vec2 texture_position;
    vec2 texture_size;
    vec4 color;
} constants;

void main() {

    if (gl_FragCoord.x < constants.screen_clip.x || gl_FragCoord.y < constants.screen_clip.y || gl_FragCoord.x > constants.screen_clip.z || gl_FragCoord.y > constants.screen_clip.w) {
        discard;
    }

    fragment_color = vec4(constants.color.rgb, texture(sprite_texture, texture_coordinates).r * constants.color.a);
}
