#version 450

layout(location = 0) out vec2 texture_coordinates_out;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 view;
    mat4 projection;
} matrices;

layout(push_constant) uniform Constants {
    mat4 world;
    vec2 texture_position;
    vec2 texture_size;
    bool mirror;
} constants;

struct Vertex {
    vec3 position;
    vec2 texture_coordinates;
};

const Vertex data[6] = Vertex[]
(
    Vertex(vec3(-1, -2, 0), vec2(1, 0)),
    Vertex(vec3(-1, 0, 0), vec2(1, 1)),
    Vertex(vec3(1, -2, 0), vec2(0, 0)),
    Vertex(vec3(1, -2, 0), vec2(0, 0)),
    Vertex(vec3(-1, 0, 0), vec2(1, 1)),
    Vertex(vec3(1, 0, 0), vec2(0, 1))
);

void main() {
    Vertex vertex = data[gl_VertexIndex];
    gl_Position = matrices.projection * matrices.view * constants.world * vec4(vertex.position, 1.0);
    texture_coordinates_out = constants.texture_position + vertex.texture_coordinates * constants.texture_size;

    if (constants.mirror) {
        texture_coordinates_out.x = 1 - texture_coordinates_out.x;
    }
}
