#version 450

const int TEXTURE_COUNT = 15;

layout(location = 0) in vec2 texture_coordinates;
layout(location = 1) flat in int texture_index;

layout (set = 1, binding = 0) uniform sampler2D textures[TEXTURE_COUNT];

void main() {

    vec4 diffuse_color;

    for (int index = 0; index < TEXTURE_COUNT; ++index)
        if (texture_index == index)
            diffuse_color = texture(textures[index], texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }
}
