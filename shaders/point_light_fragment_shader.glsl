#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput diffuse_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput normal_in;
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInput depth_in;

layout(push_constant) uniform Constants {
    mat4 screen_to_world_matrix;
    vec3 position;
    vec3 color;
    float intensity;
} constants;

void main() {

    float depth = subpassLoad(depth_in).x;

    if (depth >= 1.0) {
        discard;
    }

    vec4 pixel_position_world_space = constants.screen_to_world_matrix * vec4(position, depth, 1.0);
    pixel_position_world_space /= pixel_position_world_space.w;

    vec3 normal = normalize(subpassLoad(normal_in).rgb);
    vec3 light_direction = normalize(pixel_position_world_space.xyz - constants.position);

    float light_percent = max(dot(light_direction, normal), 0.0);
    float light_distance = length(constants.position - pixel_position_world_space.xyz);

    light_percent *= constants.intensity / exp(light_distance);

    vec3 diffuse = subpassLoad(diffuse_in).rgb;
    fragment_color.rgb = light_percent * constants.color * diffuse;
    fragment_color.a = 1.0;
}
