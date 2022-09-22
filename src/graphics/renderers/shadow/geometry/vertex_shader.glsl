#version 450

layout(location = 0) in vec3 position;
layout(location = 2) in vec2 texture_coordinates;
layout(location = 3) in int texture_index;
layout(location = 4) in float wind_affinity;

layout(location = 0) out vec2 texture_coordinates_out;
layout(location = 1) out int texture_index_out;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 view_projection;
    float time;
} matrices;

layout(push_constant) uniform Constants {
    mat4 world;
} constants;

void main() {

    vec4 world_position = constants.world * vec4(position, 1.0);
    vec4 wind_position = world_position + matrices.time;
    vec4 offset = vec4(sin(wind_position.x), 0.0, sin(wind_position.z), 0) * wind_affinity;

    gl_Position = matrices.view_projection * (world_position + offset);
    texture_coordinates_out = texture_coordinates;
    texture_index_out = texture_index;
}
