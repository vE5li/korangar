#version 450

layout(location = 0) out vec2 position_out;

const vec2 data[6] = vec2[]
(
    vec2(-1.0, -1.0),
    vec2(-1.0, 1.0),
    vec2(1.0, -1.0),
    vec2(1.0, -1.0),
    vec2(-1.0, 1.0),
    vec2(1.0, 1.0)
);

void main() {
    vec2 position = data[gl_VertexIndex];
    gl_Position = vec4(position, 0.0, 1.0);
    position_out = position;
}
