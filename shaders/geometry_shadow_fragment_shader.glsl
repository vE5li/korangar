#version 450

const int TEXTURE_COUNT = 10;

layout(location = 0) in vec2 texture_coordinates;
layout(location = 1) flat in int texture_index;

layout (set = 0, binding = 1) uniform sampler linear_sampler;
layout (set = 0, binding = 2) uniform texture2D textures[TEXTURE_COUNT];

void main() {

    vec4 diffuse_color;

    for (int index = 0; index < TEXTURE_COUNT; ++index)
        if (texture_index == index)
            diffuse_color = texture(sampler2D(textures[index], linear_sampler), texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }
}
