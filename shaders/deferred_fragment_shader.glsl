#version 450

layout(location = 0) in vec2 texture_coordinates;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec4 fragment_color;
layout(location = 1) out vec3 fragment_normal;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 world;
    mat4 view;
    mat4 projection;
} uniforms;

layout (set = 0, binding = 1) uniform sampler2D tex;
layout (set = 0, binding = 2) uniform sampler2D normal_map;
layout (set = 0, binding = 3) uniform sampler2D specular_map;

void main() {

    vec4 diffuse_color = texture(tex, texture_coordinates);

    if (diffuse_color.a == 0.0) {
        discard;
    }

    //vec4 normal_color = texture(normal_map, texture_coordinates);
    //vec4 specular_value = texture(specular_map, texture_coordinates);

    fragment_color = diffuse_color;
    fragment_normal = normal; //normal_color.xyz;
}
