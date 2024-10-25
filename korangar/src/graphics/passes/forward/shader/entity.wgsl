struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    screen_size: vec2<u32>,
    pointer_position: vec2<u32>,
    animation_timer: f32,
    day_timer: f32,
    water_level: f32,
    point_light_count: u32,
}

struct DirectionalLightUniforms {
    view_projection: mat4x4<f32>,
    color: vec4<f32>,
    direction: vec4<f32>,
}

struct PointLight {
    position: vec4<f32>,
    color: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    range: f32,
    texture_index: i32,
}

struct InstanceData {
    world: mat4x4<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    depth_offset: f32,
    curvature: f32,
    mirror: u32,
    texture_index: i32,
}

struct Vertex {
    position: vec3<f32>,
    texture_coordinates: vec2<f32>,
    depth_multiplier: f32,
    curvature_multiplier: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_position: vec4<f32>,
    @location(3) depth_offset: f32,
    @location(4) curvature: f32,
    @location(5) @interpolate(flat) original_depth_offset: f32,
    @location(6) @interpolate(flat) original_curvature: f32,
}

struct FragmentOutput {
    @location(0) fragment_color: vec4<f32>,
    @builtin(frag_depth) frag_depth: f32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var<uniform> directional_light: DirectionalLightUniforms;
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var<storage, read> point_lights: array<PointLight>;
@group(1) @binding(3) var point_shadow_maps: texture_depth_cube_array;
@group(2) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(3) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let instance = instance_data[instance_index];
    let vertex = vertex_data(vertex_index);

    var output: VertexOutput;
    let world_pos = instance.world * vec4<f32>(vertex.position, 1.0);
    output.world_position = world_pos;
    output.position = global_uniforms.view_projection * world_pos;
    output.texture_coordinates = instance.texture_position + vertex.texture_coordinates * instance.texture_size;

    if (instance.mirror != 0u) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    let rotated = rotateY(vec3<f32>(global_uniforms.view[2].x, 0, global_uniforms.view[2].z), vertex.position.x);
    output.normal = vec3<f32>(-rotated.x, rotated.y, rotated.z);
    output.depth_offset = vertex.depth_multiplier;
    output.curvature = vertex.curvature_multiplier;
    output.original_depth_offset = instance.depth_offset;
    output.original_curvature = instance.curvature;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    let diffuse_color = textureSample(texture, linear_sampler, input.texture_coordinates);
    if (diffuse_color.a != 1.0) {
        discard;
    }

    let normal = normalize(input.normal);
    let scaled_depth_offset = pow(input.depth_offset, 2.0) * input.original_depth_offset;
    let scaled_curvature_offset = (0.5 - pow(input.curvature, 2.0)) * input.original_curvature;

    // Adjust world position in view space
    let view_pos = global_uniforms.view * input.world_position;
    // TODO: NHA Why do we use this +2.0 magic offset?
    let adjusted_view_pos = view_pos - vec4<f32>(0.0, 0.0, scaled_depth_offset + scaled_curvature_offset + 2.0, 0.0);
    let adjusted_world_pos = global_uniforms.inverse_view * adjusted_view_pos;

    // Depth adjustment calculation
    let clip_pos = global_uniforms.view_projection * adjusted_world_pos;
    let clamped_depth = clamp(clip_pos.z / clip_pos.w, 0.0, 1.0);

    // Ambient light
    var final_color = diffuse_color.rgb * global_uniforms.ambient_color.rgb;

    // Directional light
    let light_direction = normalize(-directional_light.direction.xyz);
    let light_percent = dot(light_direction, normal);
    let clamped_light = clamp(light_percent, 0.0, 1.0);

    // Shadow calculation
    let light_position = directional_light.view_projection * adjusted_world_pos;
    var light_coords = light_position.xyz / light_position.w;
    let bias = clamp(0.0025 * tan(acos(clamped_light)), 0.0, 0.0005);

    let uv = clip_to_screen_space(light_coords.xy);
    let shadow_map_depth = textureSample(shadow_map, linear_sampler, uv);
    let visibility = select(0.0, 1.0, light_coords.z - bias < shadow_map_depth);

    final_color += clamped_light * directional_light.color.rgb * diffuse_color.rgb * visibility;

    // Point lights
    for (var i = 0u; i < global_uniforms.point_light_count; i++) {
        let light = point_lights[i];
        let light_direction = normalize(adjusted_world_pos.xyz - light.position.xyz);
        let light_percent = max(dot(light_direction, input.normal), 0.0);
        let light_distance = length(light.position.xyz - adjusted_world_pos.xyz);
        var visibility = 1.0;

        if (light.texture_index != 0) {
            let flipped_light_direction = vec3<f32>(light_direction.x, -light_direction.y, light_direction.z);
            let shadow_map_depth = textureSample(point_shadow_maps, linear_sampler, flipped_light_direction, light.texture_index - 1);
            let bias = clamp(0.05 * tan(acos(light_percent)), 0.0, 0.005);
            let mapped_distance = light_distance / 255.9;
            visibility = f32(mapped_distance - bias < shadow_map_depth);
        }

        let attenuation = calculate_attenuation(light_distance, light.range) * visibility;
        final_color += light_percent * light.color.rgb * diffuse_color.rgb * attenuation;
    }

    var output: FragmentOutput;
    output.fragment_color = vec4<f32>(final_color, 1.0);
    output.frag_depth = clamped_depth;
    return output;
}

// Inverse square law with smooth falloff
fn calculate_attenuation(distance: f32, range: f32) -> f32 {
    let d = min(distance, range);
    let normalized_distance = d / range;
    let att = saturate(1.0 - normalized_distance * normalized_distance);
    return att * att;
}

fn clip_to_screen_space(ndc: vec2<f32>) -> vec2<f32> {
    let u = (ndc.x + 1.0) / 2.0;
    let v = (1.0 - ndc.y) / 2.0;
    return vec2<f32>(u, v);
}

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  u  v  d  c
// 0            -1  2  1  0  0  1 -1
// 1            -1  0  1  0  1  0 -1
// 2             1  2  1  1  0  1  1
// 3             1  2  1  1  0  1  1
// 4            -1  0  1  0  1  0 -1
// 5             1  0  1  1  1  0  1
//
// (x,y,z) are the vertex position
// (u,v) are the UV coordinates
// (depth) is the depth multiplier
// (curve) is the cuvature multiplier
fn vertex_data(vertex_index: u32) -> Vertex {
    let index = 1u << vertex_index;

    let case0 = i32((index & 0x13u) != 0u);
    let case1 = i32((index & 0x0Du) != 0u);

    let x = f32(1 - 2 * case0);
    let y = f32(2 * case1);
    let z = 1.0;
    let u = f32(1 - case0);
    let v = f32(1 - case1);
    let depth = f32(case1);
    let curve = u * 2.0 - 1.0;

    return Vertex(vec3<f32>(x, y, z), vec2<f32>(u, v), depth, curve);
}


fn rotateY(direction: vec3<f32>, angle: f32) -> vec3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    let rotation_matrix = mat3x3<f32>(
        c, 0.0, -s,
        0.0, 1.0, 0.0,
        s, 0.0, c
    );
    return rotation_matrix * direction;
}
