#version 450

layout(location = 0) in vec3 normal;
layout(location = 1) in vec2 texture_coordinates;
layout(location = 2) flat in int texture_index;

layout(location = 0) out vec4 fragment_color;
layout(location = 1) out vec3 fragment_normal;

layout (set = 0, binding = 1) uniform sampler linear_sampler;
layout (set = 0, binding = 2) uniform texture2D textures[10];
//layout (set = 0, binding = 3) uniform sampler2D normal_map;
//layout (set = 0, binding = 4) uniform sampler2D specular_map;

void main() {

    vec4 diffuse_color = texture(sampler2D(textures[texture_index], linear_sampler), texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    //vec4 normal_color = texture(normal_map, texture_coordinates);
    //vec4 specular_value = texture(specular_map, texture_coordinates);

    fragment_color = diffuse_color;
    fragment_normal = normal; //normal_color.xyz;
}
