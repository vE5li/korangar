#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput diffuse_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput normal_in;
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInput depth_in;

layout(push_constant) uniform Constants {
    mat4 screen_to_world_matrix;
    bool show_diffuse_buffer;
    bool show_normal_buffer;
    bool show_depth_buffer;
} constants;

void main() {

    vec3 output_color = vec3(0.0);

    if (constants.show_diffuse_buffer) {
        vec3 diffuse = subpassLoad(diffuse_in).rgb;
        output_color += diffuse;
    }

    if (constants.show_normal_buffer) {
        vec3 normal = subpassLoad(normal_in).rgb;
        output_color += normal;
    }

    if (constants.show_depth_buffer) {
        float depth = subpassLoad(depth_in).x;

        if (depth < 1.0) {
            vec4 pixel_position_world_space = constants.screen_to_world_matrix * vec4(position, depth, 1.0);
            output_color += pixel_position_world_space.w;
        }
    }

    fragment_color = vec4(output_color, 1.0);
}
