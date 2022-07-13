#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 texture_coordinates;

layout(location = 0) out vec2 texture_coordinates_out;
layout(location = 1) out vec3 normal_out;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 view;
    mat4 projection;
} matrices;

layout(push_constant) uniform Constants {
    mat4 world;
    vec2 texture_position;
    vec2 texture_size;
} constants;

vec3 rotateY(vec3 vector, float angle) {

  float s = sin(angle);
  float c = cos(angle);
  mat3 rotation_matrix = mat3(
    c,   0.0, -s,
    0.0, 1.0, 0.0,
    s,   0.0, c
  );

  return rotation_matrix * vector;
}

void main() {
    gl_Position = matrices.projection * matrices.view * constants.world * vec4(position, 1.0);
    texture_coordinates_out = constants.texture_position + texture_coordinates * constants.texture_size;
    normal_out = rotateY(vec3(-matrices.view[2][0], 0.0, -matrices.view[2][2]), position.x);
}
