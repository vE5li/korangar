#version 450

layout(location = 0) in mat3 normal_matrix_tangentspace;
layout(location = 3) in vec2 texture_coordinates;
layout(location = 4) in vec3 vertex_position_tangentspace;

layout(location = 0) out vec4 fragment_color;

layout(set = 0, binding = 0) uniform Matrices {
    mat4 world;
    mat4 view;
    mat4 projection;
} uniforms;

layout (set = 0, binding = 1) uniform sampler2D tex;
layout (set = 0, binding = 2) uniform sampler2D normal_map;
layout (set = 0, binding = 3) uniform sampler2D specular_map;

const vec4 LIGHT = vec4(0.0, 3.0, 3.0, 1.0);

void main() {

    vec4 diffuse_color = texture(tex, texture_coordinates);
    vec4 normal_color = texture(normal_map, texture_coordinates);
    vec4 specular_value = texture(specular_map, texture_coordinates);

    float specular_reflectivity = specular_value.r * 4.0;
    vec3 specular_color = diffuse_color.rgb;

    vec3 light_position_tangentspace = normal_matrix_tangentspace * (uniforms.view * LIGHT).xyz;
    vec3 light_direction_tangentspace = normalize(light_position_tangentspace - vertex_position_tangentspace);

    vec3 normal_tangentspace = normalize(normal_color.xyz * vec3(-2.0, -2.0, 2.0) + vec3(0.4, 0.4, -1.0));

    vec3 light_color_intensity = vec3(1.0, 1.0, 1.0) * 5.0;
    float distance_from_light = distance(vertex_position_tangentspace, light_position_tangentspace);

    float diffuse_strength = clamp(dot(normal_tangentspace, light_direction_tangentspace), 0.0, 1.0);
    vec3 diffuse_light = (light_color_intensity * diffuse_strength) / (distance_from_light * distance_from_light);

    vec3 view_direction_tangentspace = normalize(vertex_position_tangentspace);
    vec3 light_reflection_tangentspace = reflect(light_direction_tangentspace, normal_tangentspace);

    float specular_lobe_factor = 5.0;
    float specular_strength = clamp(dot(view_direction_tangentspace, light_reflection_tangentspace), 0.0, 1.0);
    vec3 specular_light = (light_color_intensity * pow(specular_strength, specular_lobe_factor)) / (distance_from_light * distance_from_light);

    fragment_color.rgb = (diffuse_color.rgb * diffuse_light) + (specular_color * specular_reflectivity * specular_light);
    fragment_color.a = diffuse_color.a;
}
