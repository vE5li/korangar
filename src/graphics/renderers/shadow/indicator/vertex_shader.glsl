#version 450

layout(location = 0) out vec2 texture_coordinates;

layout(push_constant) uniform Constants {
    mat4 view_projection;
    vec3 upper_left;
    vec3 upper_right;
    vec3 lower_left;
    vec3 lower_right;
} constants;

const vec2 texture_coordinates_lookup[6] = vec2[]
(
    vec2(0.0, 0.0),
    vec2(0.0, 1.0),
    vec2(1.0, 0.0),
    vec2(1.0, 0.0),
    vec2(0.0, 1.0),
    vec2(1.0, 1.0)
);

void main() {
    vec3 vertex_lookup[6] = vec3[]
    (
         constants.upper_left,
         constants.upper_right,
         constants.lower_left,
         constants.lower_left,
         constants.upper_right,
         constants.lower_right
    );

    gl_Position = constants.view_projection * vec4(vertex_lookup[gl_VertexIndex], 1.0);
    texture_coordinates = texture_coordinates_lookup[gl_VertexIndex];
}
