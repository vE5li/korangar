struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
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
    range: f32,
    texture_index: i32,
}

struct TileLightIndices {
    indices: array<u32, 256>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
}

const TILE_SIZE: u32 = 16;

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var<uniform> directional_light: DirectionalLightUniforms;
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var<storage, read> point_lights: array<PointLight>;
@group(1) @binding(3) var light_count_texture: texture_2d<u32>;
@group(1) @binding(4) var<storage, read> tile_light_indices: array<TileLightIndices>;
@group(1) @binding(5) var point_shadow_maps: texture_depth_cube_array;
@group(2) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let world_position = position_data(vertex_index);

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * world_position;
    output.world_position = world_position;
    output.normal = normal_data(vertex_index);
    output.texture_coordinates = uv_data(vertex_index);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel_color = textureSample(texture, nearest_sampler, input.texture_coordinates);

    // Calculate which tile this fragment belongs to
    let pixel_position = vec2<u32>(floor(input.position.xy));
    let tile_x = pixel_position.x / TILE_SIZE;
    let tile_y = pixel_position.y / TILE_SIZE;
    let tile_count_x = (global_uniforms.screen_size.x + TILE_SIZE - 1u) / TILE_SIZE;
    let tile_index = tile_y * tile_count_x + tile_x;

    // Get the number of lights affecting this tile
    let light_count = textureLoad(light_count_texture, vec2<u32>(tile_x, tile_y), 0).r;

    if (texel_color.a < 0.1) {
        discard;
    }

    var base_color = texel_color.rgb * global_uniforms.indicator_color.rgb;

    // Directional light
    let light_percent = clamp(dot(normalize(-directional_light.direction.xyz), input.normal), 0.0, 1.0);
    let bias = clamp(0.0025 * tan(acos(light_percent)), 0.0, 0.0005);

    let light_position = directional_light.view_projection * input.world_position;
    let light_coords = light_position.xyz / light_position.w;
    let uv = clip_to_screen_space(light_coords.xy);
    let shadow_map_depth = textureSample(shadow_map, nearest_sampler, uv);
    let visibility = select(0.0, 1.0, light_coords.z - bias < shadow_map_depth);
    let directional_light = light_percent * directional_light.color.rgb * base_color * visibility;

    // Combine base color, ambient light and directional light
    var final_color = base_color * global_uniforms.ambient_color.rgb + directional_light;

    // Point lights
    for (var index = 0u; index < light_count; index++) {
        let light_index = tile_light_indices[tile_index].indices[index];
        let light = point_lights[light_index];
        let light_direction = normalize(input.world_position.xyz - light.position.xyz);
        let light_percent = max(dot(light_direction, input.normal), 0.0);
        let light_distance = length(light.position.xyz - input.world_position.xyz);
        var visibility = 1.0;

        if (light.texture_index != 0) {
            let flipped_light_direction = vec3<f32>(light_direction.x, -light_direction.y, light_direction.z);
            let shadow_map_depth = textureSample(point_shadow_maps, nearest_sampler, flipped_light_direction, light.texture_index - 1);
            let bias = clamp(0.05 * tan(acos(light_percent)), 0.0, 0.005);
            let mapped_distance = light_distance / 255.9;
            visibility = f32(mapped_distance - bias < shadow_map_depth);
        }

        let attenuation = calculate_attenuation(light_distance, light.range) * visibility;
        final_color += light_percent * light.color.rgb * base_color * attenuation;
    }

    return vec4<f32>(final_color, texel_color.a);
}

// Quadratic attenuation with smooth falloff
fn calculate_attenuation(distance: f32, range: f32) -> f32 {
    let effective_distance = min(distance, range);
    let normalized_distance = effective_distance / range;
    let attenuation = saturate(1.0 - normalized_distance * normalized_distance);
    return attenuation * attenuation;
}

fn clip_to_screen_space(ndc: vec2<f32>) -> vec2<f32> {
    let u = (ndc.x + 1.0) / 2.0;
    let v = (1.0 - ndc.y) / 2.0;
    return vec2<f32>(u, v);
}

fn position_data(vertex_index: u32) -> vec4<f32> {
    switch (vertex_index) {
        case 0u: {
            // upper_left
            return global_uniforms.indicator_positions[0u];
        }
        case 1u: {
            // upper_right
            return global_uniforms.indicator_positions[1u];
        }
        case 2u: {
            // lower_left
            return global_uniforms.indicator_positions[2u];
        }
        case 3u: {
            // lower_left
            return global_uniforms.indicator_positions[2u];
        }
        case 4u: {
            // upper_right
            return global_uniforms.indicator_positions[1u];
        }
        default: {
            // lower_right
            return global_uniforms.indicator_positions[3u];
        }
    }
}

// Optimized version of the following truth table:
//
// vertex_index  x  y
// 0             0  0
// 1             0  1
// 2             1  0
// 3             1  0
// 4             0  1
// 5             1  1
//
// (x,y) are the UV coordinates
fn uv_data(vertex_index: u32) -> vec2<f32> {
    let index = 1u << vertex_index;
    let x = f32((index & 0x2Cu) != 0u);
    let y = f32((index & 0x32u) != 0u);
    return vec2<f32>(x, y);
}

fn normal_data(vertex_index: u32) -> vec3<f32> {
    if (vertex_index < 3) {
        return normalize(cross(
            // upper_right - upper_left, lower_left - upper_left
            (global_uniforms.indicator_positions[1] - global_uniforms.indicator_positions[0]).xyz,
            (global_uniforms.indicator_positions[2] - global_uniforms.indicator_positions[0]).xyz
        ));
    } else {
        return normalize(cross(
            // upper_right - lower_left, lower_right - lower_left
            (global_uniforms.indicator_positions[1] - global_uniforms.indicator_positions[2]).xyz,
            (global_uniforms.indicator_positions[3] - global_uniforms.indicator_positions[2]).xyz
        ));
    }
}

