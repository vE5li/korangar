#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInputMS diffuse_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInputMS normal_in;
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInputMS depth_in;

layout (set = 0, binding = 3) uniform sampler2D shadow_map_in;

layout(set = 0, binding = 4) uniform Matrices {
    mat4 screen_to_world;
    mat4 light;
} matrices;

layout(push_constant) uniform Constants {
    vec3 direction;
    vec3 color;
} constants;

vec3 calculate_sample(int sample_index) {

    float depth = subpassLoad(depth_in, sample_index).x;
    vec4 pixel_position_world_space = matrices.screen_to_world * vec4(position, depth, 1.0);
    pixel_position_world_space /= pixel_position_world_space.w;

    vec3 normal = normalize(subpassLoad(normal_in, sample_index).rgb);
    float light_percent = dot(normalize(-constants.direction), normal);
    light_percent = clamp(light_percent, 0.0, 1.0);

    // triangles flicker black if the direction of the light is the exact opposite of the normal of the triangle
    float bias = 0.005 * tan(acos(light_percent));
    bias = clamp(bias, 0 ,0.01);

    vec4 light_position = matrices.light * pixel_position_world_space;
    vec3 light_coords = light_position.xyz / light_position.w;
    light_coords.xy = light_coords.xy * 0.5 + 0.5;

    float shadow_map_depth = texture(shadow_map_in, light_coords.xy).r;
    bool visibility = light_coords.z - bias < shadow_map_depth;

    vec3 diffuse = subpassLoad(diffuse_in, sample_index).rgb;
    return light_percent * constants.color * diffuse * float(visibility); 
}

void main() {

    vec3 blended = vec3(0.0);

    for (int i = 0; i < 4; i++)
        blended += calculate_sample(i);

    fragment_color.rgb = blended / 4.0;
    fragment_color.a = 1.0;
}
