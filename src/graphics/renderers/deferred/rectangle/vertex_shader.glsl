#version 450

//layout(location = 0) in vec2 position;

layout(push_constant) uniform Constants {
    vec2 screen_position;
    vec2 screen_size;
    vec4 color;
} constants;

const vec2 data[6] = vec2[]
(
    vec2(0, 0),
    vec2(0, 1),
    vec2(1, 0),
    vec2(1, 0),
    vec2(0, 1),
    vec2(1, 1)
);

void main() {
    vec2 position = data[gl_VertexIndex];
    vec2 vertex_position = constants.screen_position - vec2(1.0) + position * constants.screen_size;
    gl_Position = vec4(vertex_position, 0.0, 1.0);
}
