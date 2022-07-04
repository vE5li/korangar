#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInputMS diffuse_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInputMS normal_in;
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInputMS depth_in;

layout(push_constant) uniform Constants {
    mat4 screen_to_world_matrix;
    vec2 screen_position;
    vec2 screen_size;
    vec3 position;
    vec3 color;
    float range;
} constants;

vec3 calculate_sample(int sample_index) {

    float depth = subpassLoad(depth_in, sample_index).x;

    vec4 pixel_position_world_space = constants.screen_to_world_matrix * vec4(position, depth, 1.0);
    pixel_position_world_space /= pixel_position_world_space.w;

    vec3 normal = normalize(subpassLoad(normal_in, sample_index).rgb);
    vec3 light_direction = normalize(pixel_position_world_space.xyz - constants.position);

    float light_percent = max(dot(light_direction, normal), 0.0);
    float light_distance = length(constants.position - pixel_position_world_space.xyz);

    light_percent *= constants.range / exp(light_distance / 10.0);

    vec3 diffuse = subpassLoad(diffuse_in, sample_index).rgb;
    return min(light_percent * constants.color, vec3(0.6)) * diffuse;
}

void main() {

    vec3 blended = vec3(0.0);

    for (int i = 0; i < 4; i++)
        blended += calculate_sample(i);

    fragment_color.rgb = blended / 4.0;
    fragment_color.a = 1.0;
}
