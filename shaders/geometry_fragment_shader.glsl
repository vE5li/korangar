#version 450

const int TEXTURE_COUNT = 15;

layout(location = 0) in vec3 normal;
layout(location = 1) in vec2 texture_coordinates;
layout(location = 2) flat in int texture_index;

layout(location = 0) out vec4 fragment_color;
layout(location = 1) out vec3 fragment_normal;

layout (set = 0, binding = 1) uniform sampler2D textures[15];

void main() {

    vec4 diffuse_color;

    for (int index = 0; index < TEXTURE_COUNT; ++index)
        if (texture_index == index)
            diffuse_color = texture(textures[index], texture_coordinates);

    //diffuse_color = texture(sampler2D(textures[texture_index], linear_sampler), texture_coordinates);

    if (diffuse_color.a < 0.5) {
        discard;
    } 
    
    fragment_color = diffuse_color;

    fragment_color.r = pow(fragment_color.r, 1.0 / 1.8);
    fragment_color.g = pow(fragment_color.g, 1.0 / 1.8);
    fragment_color.b = pow(fragment_color.b, 1.0 / 1.8);

    fragment_normal = normal; //normal_color.xyz;
}
