struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
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
    inv_world: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(3) color: vec3<f32>,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(2) var linear_sampler: sampler;
@group(0) @binding(3) var texture_sampler: sampler;
@group(1) @binding(0) var<uniform> directional_light: DirectionalLightUniforms;
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var<storage, read> point_lights: array<PointLight>;
@group(1) @binding(3) var point_shadow_maps: texture_depth_cube_array;
@group(2) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(3) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(3) color: vec3<f32>,
    @location(4) wind_affinity: f32,
    @location(5) instance_id: u32
) -> VertexOutput {
    let instance = instance_data[instance_id];

    let world_position = instance.world * vec4<f32>(position, 1.0);
    let wind_position = world_position + vec4<f32>(global_uniforms.animation_timer);
    let offset = vec4<f32>(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * wind_affinity;
    let final_world_pos = world_position + offset;

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * final_world_pos;
    output.world_position = final_world_pos;
    output.normal = normalize((instance.inv_world * vec4<f32>(normal, 0.0)).xyz);
    output.texture_coordinates = texture_coordinates;
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let diffuse_color = textureSample(texture, texture_sampler, input.texture_coordinates);
    let alpha_channel = textureSample(texture, nearest_sampler, input.texture_coordinates).a;

    if (alpha_channel < 1.0) {
        discard;
    }

    var base_color = diffuse_color.rgb * input.color;

    // Directional light
    let light_percent = clamp(dot(normalize(-directional_light.direction.xyz), input.normal), 0.0, 1.0);
    let bias = clamp(0.0025 * tan(acos(light_percent)), 0.0, 0.0005);

    let light_position = directional_light.view_projection * input.world_position;
    let light_coords = light_position.xyz / light_position.w;
    let uv = clip_to_screen_space(light_coords.xy);
    let shadow_map_depth = textureSample(shadow_map, linear_sampler, uv);
    let visibility = select(0.0, 1.0, light_coords.z - bias < shadow_map_depth);
    let directional_light = light_percent * directional_light.color.rgb * base_color * visibility;

    // Combine base color, ambient light and directional light
    var final_color = base_color * global_uniforms.ambient_color.rgb + directional_light;

    // Point lights
    for (var i = 0u; i < global_uniforms.point_light_count; i++) {
        let light = point_lights[i];
        let light_direction = normalize(input.world_position.xyz - light.position.xyz);
        let light_percent = max(dot(light_direction, input.normal), 0.0);
        let light_distance = length(light.position.xyz - input.world_position.xyz);
        var visibility = 1.0;

        if (light.texture_index != 0) {
            let flipped_light_direction = vec3<f32>(light_direction.x, -light_direction.y, light_direction.z);
            let shadow_map_depth = textureSample(point_shadow_maps, linear_sampler, flipped_light_direction, light.texture_index - 1);
            let bias = clamp(0.05 * tan(acos(light_percent)), 0.0, 0.005);
            let mapped_distance = light_distance / 255.9;
            visibility = f32(mapped_distance - bias < shadow_map_depth);
        }

        let attenuation = min(light.range / exp(light_distance / 10.0), 0.7) * visibility;
        final_color += light_percent * light.color.rgb * base_color * attenuation;
    }

    return vec4<f32>(final_color, 1.0);
}

fn clip_to_screen_space(ndc: vec2<f32>) -> vec2<f32> {
    let u = (ndc.x + 1.0) / 2.0;
    let v = (1.0 - ndc.y) / 2.0;
    return vec2<f32>(u, v);
}
