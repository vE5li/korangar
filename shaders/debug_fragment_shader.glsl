#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInputMS diffuse_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInputMS normal_in;
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInputMS water_in;
layout(input_attachment_index = 3, set = 0, binding = 3) uniform subpassInputMS depth_in;

layout(set = 0, binding = 4) uniform usampler2D picker_buffer;

layout(push_constant) uniform Constants {
    mat4 screen_to_world_matrix;
    bool show_diffuse_buffer;
    bool show_normal_buffer;
    bool show_water_buffer;
    bool show_depth_buffer;
    bool show_picker_buffer;
} constants;

void main() {

    vec3 output_color = vec3(0.0);

    if (constants.show_diffuse_buffer) {
        vec3 diffuse = subpassLoad(diffuse_in, 0).rgb;
        output_color += diffuse;
    }

    if (constants.show_normal_buffer) {
        vec3 normal = subpassLoad(normal_in, 0).rgb;
        output_color += normal;
    }

    if (constants.show_water_buffer) {
        float water = subpassLoad(water_in, 0).r;
        output_color += vec3(0, 0, water);
    }

    if (constants.show_depth_buffer) {
        float depth = subpassLoad(depth_in, 0).x;

        if (depth < 1.0) {
            vec4 pixel_position_world_space = constants.screen_to_world_matrix * vec4(position, depth, 1.0);
            output_color += pixel_position_world_space.w;
        }
    }

    if (constants.show_picker_buffer) {
        uint picker = texture(picker_buffer, position * 0.5 + 0.5).r;
        output_color += vec3(picker);
    }

    //if (constants.show_directional_shadow_map) {
    //    float depth = subpassLoad(directional_shadow_map_in).x;

    //    if (depth < 1.0) {
    //        vec4 pixel_position_world_space = constants.screen_to_world_matrix * vec4(position, depth, 1.0);
    //        output_color += pixel_position_world_space.w;
    //    }
    //}

    fragment_color = vec4(output_color, 1.0);
}
