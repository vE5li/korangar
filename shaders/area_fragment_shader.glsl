#version 450

const int TEXTURE_COUNT = 10;

layout(location = 0) in vec3 normal;

layout(location = 0) out vec4 fragment_color;
layout(location = 1) out vec3 fragment_normal;

void main() {

    fragment_color = vec4(1.0, 0.0, 0.0, 1.0);
    fragment_normal = normal;
}
