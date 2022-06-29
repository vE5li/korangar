#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInputMS water_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInputMS depth_in;

layout(push_constant) uniform Constants {
    mat4 screen_to_world_matrix;
    float water_level;
} constants;

vec3 calculate_sample(int sample_index) {

    float water = subpassLoad(water_in, sample_index).r;
    float depth = subpassLoad(depth_in, sample_index).x;

    vec4 pixel_position_world_space = constants.screen_to_world_matrix * vec4(position, depth, 1.0);
    pixel_position_world_space /= pixel_position_world_space.w;

    float delta = constants.water_level - pixel_position_world_space.y;
    float factor = max(0.2, delta / 30);

    return vec3(factor, min(factor / 2, 0.3), min(factor / 10, 0.1)) * water;
}

void main() {

    vec3 blended = vec3(0.0);

    for (int i = 0; i < 4; i++)
        blended += calculate_sample(i);

    fragment_color.rgb = blended / 4.0;
    fragment_color.a = 1.0;

    /*float water = subpassLoad(water_in, 0).r;

    if (water == 0.0) {
        discard;
    }

    float depth = subpassLoad(depth_in, 0).x;

    vec4 pixel_position_world_space = constants.screen_to_world_matrix * vec4(position, depth, 1.0);
    pixel_position_world_space /= pixel_position_world_space.w;

    float delta = constants.water_level - pixel_position_world_space.y;
    float jonas = max(0.2, delta / 30);

    fragment_color.rgb = vec3(jonas, min(jonas / 2, 0.3), min(jonas / 10, 0.1));
    fragment_color.a = 1.0;*/
}
