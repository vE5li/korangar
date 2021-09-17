#version 450

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput diffuse_in;

layout(push_constant) uniform Constants {
    vec3 color;
} constants;

void main() {
    vec3 diffuse = subpassLoad(diffuse_in).rgb;
    fragment_color.rgb = diffuse * constants.color;
    fragment_color.a = 1.0;
}
