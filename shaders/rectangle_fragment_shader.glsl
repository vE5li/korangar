#version 450

layout(location = 0) in vec2 fragment_position;

layout(location = 3) out vec4 fragment_color;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    vec2 clip_size;
    vec4 corner_radius;
    vec3 color;
} constants;

void main() {

    vec2 coords = fragment_position * constants.screen_size;

    // top-left
    if (length(coords - constants.corner_radius.x) > constants.corner_radius.x && coords.x < constants.corner_radius.x &&
        coords.y < constants.corner_radius.x) {
        discard;
    } 

    // top-right
    if (length(coords - vec2(constants.screen_size.x - constants.corner_radius.y, constants.corner_radius.y)) > constants.corner_radius.y &&
        constants.screen_size.x - coords.x < constants.corner_radius.y && coords.y < constants.corner_radius.y) {
        discard;
    } 

    // bottom-right
    if (length(coords - constants.screen_size + constants.corner_radius.z) > constants.corner_radius.z &&
        constants.screen_size.x - coords.x < constants.corner_radius.z && constants.screen_size.y - coords.y < constants.corner_radius.z) {
        discard;
    } 

    // bottom_left
    if (length(coords - vec2(constants.corner_radius.w, constants.screen_size.y - constants.corner_radius.w)) > constants.corner_radius.w &&
        coords.x < constants.corner_radius.w && constants.screen_size.y - coords.y < constants.corner_radius.w) {
        discard;
    } 

    if (gl_FragCoord.x > constants.clip_size.x || gl_FragCoord.y > constants.clip_size.y) {
        discard;
    }

    fragment_color = vec4(constants.color, 1.0);

    //fragment_color.r = pow(fragment_color.r, 0.2);
    //fragment_color.g = pow(fragment_color.g, 0.2);
    //fragment_color.b = pow(fragment_color.b, 0.2);
}
