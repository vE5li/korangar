#version 450

layout(location = 0) in vec2 fragment_position;

layout(location = 0) out vec4 fragment_color;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    vec4 clip_size;
    vec4 corner_radius;
    vec4 color;
    float aspect_ratio;
} constants;

void main() {

    vec2 coords = fragment_position * constants.screen_size;
    vec2 screen_size = constants.screen_size;

    coords.x /= constants.aspect_ratio;
    screen_size.x /= constants.aspect_ratio;

    // top-left
    if (length(coords - constants.corner_radius.x) > constants.corner_radius.x && coords.x < constants.corner_radius.x &&
        coords.y < constants.corner_radius.x) {
        discard;
    }

    // top-right
    if (length(coords - vec2(screen_size.x - constants.corner_radius.y, constants.corner_radius.y)) > constants.corner_radius.y &&
        screen_size.x - coords.x < constants.corner_radius.y && coords.y < constants.corner_radius.y) {
        discard;
    }

    // bottom-right
    if (length(coords - screen_size + constants.corner_radius.z) > constants.corner_radius.z &&
        screen_size.x - coords.x < constants.corner_radius.z && screen_size.y - coords.y < constants.corner_radius.z) {
        discard;
    }

    // bottom_left
    if (length(coords - vec2(constants.corner_radius.w, screen_size.y - constants.corner_radius.w)) > constants.corner_radius.w &&
        coords.x < constants.corner_radius.w && screen_size.y - coords.y < constants.corner_radius.w) {
        discard;
    }

    if (gl_FragCoord.x < constants.clip_size.x || gl_FragCoord.y < constants.clip_size.y || gl_FragCoord.x > constants.clip_size.z || gl_FragCoord.y > constants.clip_size.w) {
        discard;
    }

    fragment_color = constants.color;
}
