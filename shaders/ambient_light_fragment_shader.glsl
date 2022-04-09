#version 450

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput diffuse_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput normal_in;

layout(push_constant) uniform Constants {
    vec3 color;
} constants;

void main() {

    vec3 diffuse = subpassLoad(diffuse_in).rgb;
    vec3 normal = subpassLoad(normal_in).rgb;
    fragment_color.a = 1.0;

    if (length(normal) == 0.0) {
        fragment_color.rgb = diffuse;
        return;
    }

    fragment_color.rgb = diffuse * constants.color;
}
