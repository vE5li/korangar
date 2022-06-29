#version 450

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInputMS diffuse_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInputMS normal_in;

layout(push_constant) uniform Constants {
    vec3 color;
} constants;

vec3 calculate_sample(int sample_index) {

    vec3 diffuse = subpassLoad(diffuse_in, sample_index).rgb;
    vec3 normal = subpassLoad(normal_in, sample_index).rgb;

    if (length(normal) == 0.0) {
        return diffuse;
    }

    return diffuse * constants.color;
}

void main() {

    vec3 blended = vec3(0.0);

    for (int i = 0; i < 4; i++)
        blended += calculate_sample(i);

    fragment_color.rgb = blended / 4.0;
    fragment_color.a = 1.0;

    //vec3 diffuse = subpassLoad(diffuse_in, 0).rgb;
    //vec3 normal = subpassLoad(normal_in, 0).rgb;
    //fragment_color.a = 1.0;

    //if (length(normal) == 0.0) {
    //    fragment_color.rgb = diffuse;
    //    return;
    //}

    //fragment_color.rgb = diffuse * constants.color;
}
