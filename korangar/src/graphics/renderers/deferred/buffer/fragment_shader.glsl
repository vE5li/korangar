#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInputMS diffuse_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInputMS normal_in;
layout(input_attachment_index = 2, set = 0, binding = 2) uniform subpassInputMS water_in;
layout(input_attachment_index = 3, set = 0, binding = 3) uniform subpassInputMS depth_in;

layout(set = 0, binding = 4) uniform usampler2D picker_buffer;
layout(set = 0, binding = 5) uniform sampler2D shadow_buffer;
layout(set = 0, binding = 6) uniform sampler2D font_atlas;

layout(push_constant) uniform Constants {
    bool show_diffuse_buffer;
    bool show_normal_buffer;
    bool show_water_buffer;
    bool show_depth_buffer;
    bool show_picker_buffer;
    bool show_shadow_buffer;
    bool show_font_atlas;
} constants;

float linearize(in float rawValue, in float zNear, in float zFar) {
    return (2.0 * zNear) / (zFar + zNear - rawValue * (zFar - zNear));
}

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
        vec4 water = subpassLoad(water_in, 0);
        output_color += vec3(0, 0, water.r);
    }

    if (constants.show_depth_buffer) {
        float depth = subpassLoad(depth_in, 0).x;
        output_color += linearize(depth, 1.0, 2000.0);
    }

    if (constants.show_picker_buffer) {
        uint picker = texture(picker_buffer, position * 0.5 + 0.5).r;
        float red = (picker & 0xf) / 100.0;
        float green = (picker >> 8 & 0xf) / 100.0;
        float blue = (picker >> 16 & 0xf) / 100.0;
        output_color += vec3(red, green, blue);
    }

    if (constants.show_shadow_buffer) {
        vec2 sample_position = (position * 0.5 + 0.5);
        sample_position.y = 1.0 - sample_position.y;
        float depth = texture(shadow_buffer, sample_position).x;
        output_color += depth;
    }

    if (constants.show_font_atlas) {
        float color = texture(font_atlas, position * 0.5 + 0.5).r;
        output_color += color;
    }

    fragment_color = vec4(output_color, 1.0);
}
