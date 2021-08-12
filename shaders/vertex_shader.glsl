#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 tangent;
layout(location = 3) in vec3 bitangent;
layout(location = 4) in vec2 texture_coordinates;
layout(location = 0) out mat3 normal_matrix_tangentspace;
layout(location = 3) out vec2 texture_coordinates_out;
layout(location = 4) out vec3 vertex_position_tangentspace;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 world;
    mat4 view;
    mat4 projection;
} uniforms;

void main() {
    mat4 worldview = uniforms.view * uniforms.world;  
    vec4 vertex_position_worldspace = uniforms.world * vec4(position, 1.0);
    vec4 vertex_position_viewspace = uniforms.view * vertex_position_worldspace;

    gl_Position = uniforms.projection * vertex_position_viewspace;
    normal_matrix_tangentspace = transpose(mat3(worldview) * mat3(tangent, bitangent, normal));
    //vertex_position_tangentspace = normalize(normal_matrix_tangentspace * vertex_position_viewspace.xyz);
    vertex_position_tangentspace = normal_matrix_tangentspace * vertex_position_viewspace.xyz;
    texture_coordinates_out = texture_coordinates;
}
