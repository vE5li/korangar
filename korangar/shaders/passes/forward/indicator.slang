#language slang 2026

import depth_texture;
import globals;
import forward;
import matrix;
import transform;
import sdsm;

struct IndicatorVertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] var world_position: float4;
    [[vk::location(1)]] var normal: float3;
    [[vk::location(2)]] var texture_coordinates: float2;
};

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(1, 0)]] var nearest_sampler: SamplerState;
[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(4, 0)]] var shadow_map_sampler: SamplerComparisonState;
[[vk::binding(0, 1)]] var directional_light: ConstantBuffer<DirectionalLightUniforms>;
[[vk::binding(1, 1)]] var shadow_maps: DepthTexture2DArray;
[[vk::binding(2, 1)]] var point_lights: StructuredBuffer<PointLight>;
[[vk::binding(3, 1)]] var light_count_texture: Texture2D<uint>;
[[vk::binding(4, 1)]] var tile_light_indices: StructuredBuffer<TileLightIndices>;
[[vk::binding(5, 1)]] var point_shadow_maps: DepthTextureCubeArray;
[[vk::binding(6, 1)]] var directional_light_partitions: StructuredBuffer<DirectionalLightPartition>;
[[vk::binding(7, 1)]] var kernel_uniforms: ConstantBuffer<KernelUniforms>;
[[vk::binding(8, 1)]] var shadow_translucence: Texture2DArray;
[[vk::binding(0, 2)]] var texture: Texture2D;

static const var INDICATOR_CONSTANT_DEPTH_BIAS: float = 0.0005;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> IndicatorVertexOutput {
    let world_position = position_data(vertex_index);

    var output: IndicatorVertexOutput;
    output.position = mul(global_uniforms.view_projection, world_position);
    output.world_position = world_position;
    output.normal = normal_data(vertex_index);
    output.texture_coordinates = uv_data(vertex_index);
    return output;
}

[[shader("pixel")]]
func fs_main(input: IndicatorVertexOutput) -> float4 {
    let diffuse_color = texture.Sample(nearest_sampler, input.texture_coordinates);

    // Calculate which tile this fragment belongs to.
    let pixel_position = uint2(floor(input.position.xy));
    let tile_x = pixel_position.x / TILE_SIZE;
    let tile_y = pixel_position.y / TILE_SIZE;
    let tile_count_x = (global_uniforms.forward_size.x + TILE_SIZE - 1) / TILE_SIZE;
    let tile_index = tile_y * tile_count_x + tile_x;

    // Get the number of lights affecting this tile.
    let light_count = light_count_texture.Load(int3(tile_x, tile_y, 0)).r;

    if (diffuse_color.a < 0.1) {
        discard;
    }

    let normal = normalize(input.normal);

    // Ambient light
    var ambient_light_contribution = global_uniforms.ambient_color.rgb;

    // Directional light
    let light_direction = normalize(-directional_light.direction.xyz);
    let light_percent = max(dot(light_direction, normal), 0.0);

    // Shadow calculation
    let linear_view_z = depth::nonLinearToLinear(input.position.z);

    var partition_index: uint = 0;
    [unroll]
    for (var i: uint = 0; i < (PARTITION_COUNT - 1); i++) {
        if (linear_view_z >= directional_light_partitions[i].interval_end) {
            partition_index++;
        }
    }

    let shadow_position = mul(directional_light_partitions[partition_index].view_projection, input.world_position);
    var shadow_coords = shadow_position.xyz / shadow_position.w;
    shadow_coords = float3(
        coordinate_space::clip_to_screen_space(shadow_coords.xy),
        // If the far plane of the shadow is in front of the drawn object, then it's value would be below 0.0.
        // That would create a false shadow, so we need to make sure the value is never below 0.0 so that this
        // phantom shadowing is not occurring (can happen when zooming).
        max(shadow_coords.z, 0.0) + INDICATOR_CONSTANT_DEPTH_BIAS
    );

    var visibility: float = 1.0;

    switch (global_uniforms.shadow_method) {
        case 0: {
            visibility = shadow_maps.SampleCmp(
                shadow_map_sampler,
                float3(shadow_coords.xy, partition_index),
                shadow_coords.z
            );
            break;
        }
        case 1: {
            uint shadow_width, shadow_height, shadow_depth;
            shadow_maps.GetDimensions(shadow_width, shadow_height, shadow_depth);
            let shadow_map_dimensions = uint2(shadow_width, shadow_height);
            visibility = get_pcf_shadow(partition_index, shadow_coords, shadow_map_dimensions, global_uniforms.shadow_detail, shadow_maps, shadow_map_sampler);
            break;
        }
        case 2: {
            visibility = get_pcf_pcss_shadow(partition_index, shadow_coords, global_uniforms, directional_light_partitions, shadow_maps, shadow_map_sampler, linear_sampler, kernel_uniforms);
            break;
        }
    }

    let translucence = shadow_translucence.SampleLevel(linear_sampler, float3(shadow_coords.xy, partition_index), 0.0).r;
    visibility *= translucence;

    let directional_light_contribution = directional_light.color.rgb * light_percent * visibility;

    // Point lights
    var point_light_contribution = float3(0.0);
    for (var index = 0; index < light_count; index++) {
        let light_index = tile_light_indices[tile_index].indices[index];
        let light = point_lights[light_index];
        let light_direction_point = normalize(input.world_position.xyz - light.position.xyz);
        let light_percent_point = max(dot(light_direction_point, input.normal), 0.0);
        let light_distance = length(light.position.xyz - input.world_position.xyz);
        var visibility_point = 1.0;

        if (light.texture_index != 0) {
            let bias = 1.2;
            let distance_to_light = depth::linearToNonLinear(light_distance - bias);

            let closest_distance = point_shadow_maps.Sample(
                linear_sampler,
                float4(light_direction_point, light.texture_index - 1)
            );

            visibility_point = float(distance_to_light > closest_distance);
        }

        let intensity = 10.0;
        let attenuation = calculate_attenuation(light_distance, light.range);
        point_light_contribution += (light.color.rgb * intensity) * light_percent_point * attenuation * visibility_point;
    }

    let base_color = diffuse_color * global_uniforms.indicator_color;
    let light_contributions = saturate(ambient_light_contribution + directional_light_contribution + point_light_contribution);
    var color = base_color.rgb * light_contributions;

    if (global_uniforms.enhanced_lighting == 0) {
        color = color_balance(color, -0.01, 0.0, 0.0);
    }

    return float4(color, base_color.a);
}

[ForceInline]
func position_data(vertex_index: uint) -> float4 {
    switch (vertex_index) {
        case 0:
            // upper_left
            return global_uniforms.indicator_positions.getColumn(0);
        case 1:
            // upper_right
            return global_uniforms.indicator_positions.getColumn(1);
        case 2:
            // lower_left
            return global_uniforms.indicator_positions.getColumn(2);
        case 3:
            // lower_left
            return global_uniforms.indicator_positions.getColumn(2);
        case 4:
            // upper_right
            return global_uniforms.indicator_positions.getColumn(1);
        default:
            // lower_right
            return global_uniforms.indicator_positions.getColumn(3);
    }
}

[ForceInline]
func normal_data(vertex_index: uint) -> float3 {
    if (vertex_index < 3) {
        return normalize(cross(
            // upper_right - upper_left, lower_left - upper_left
            (global_uniforms.indicator_positions.getColumn(1) - global_uniforms.indicator_positions.getColumn(0)).xyz,
            (global_uniforms.indicator_positions.getColumn(2) - global_uniforms.indicator_positions.getColumn(0)).xyz
        ));
    } else {
        return normalize(cross(
            // upper_right - lower_left, lower_right - lower_left
            (global_uniforms.indicator_positions.getColumn(1) - global_uniforms.indicator_positions.getColumn(2)).xyz,
            (global_uniforms.indicator_positions.getColumn(3) - global_uniforms.indicator_positions.getColumn(2)).xyz
        ));
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
[ForceInline]
func uv_data(vertex_index: uint) -> float2 {
    let index = 1 << vertex_index;
    let x = float((index & 0x2C) != 0);
    let y = float((index & 0x32) != 0);
    return float2(x, y);
}
