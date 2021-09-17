#version 450

layout(location = 0) out vec4 fragment_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput diffuse_in;
layout(input_attachment_index = 1, set = 0, binding = 1) uniform subpassInput normal_in;

layout(push_constant) uniform Constants {
    vec3 direction;
    vec3 color;
} constants;

void main() {

    vec3 normal = normalize(subpassLoad(normal_in).rgb);
    float light_percent = dot(normalize(constants.direction), normal);

    light_percent = max(light_percent, 0.0);

    vec3 diffuse = subpassLoad(diffuse_in).rgb;
    fragment_color.rgb = light_percent * constants.color * diffuse;
    fragment_color.a = 1.0;
}
