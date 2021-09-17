#version 450

layout(location = 0) in vec2 position;

layout(location = 0) out vec2 position_out;

void main() {
    gl_Position = vec4(position, 0.0, 1.0);
    position_out = position;
}
