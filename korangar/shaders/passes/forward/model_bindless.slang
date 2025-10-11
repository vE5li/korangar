#language slang 2026

import depth_texture;
import globals;
import forward;
import matrix;
import transform;
import sdsm;

struct ModelBindlessVertexInput {
    uint instance_index : SV_VulkanInstanceID;
    [[vk::location(0)]] var position: float3;
    [[vk::location(1)]] var normal: float3;
    [[vk::location(2)]] var texture_coordinates: float2;
    [[vk::location(3)]] var color: float3;
    [[vk::location(4)]] var texture_index: int;
    [[vk::location(5)]] var wind_affinity: float;
}

struct ModelBindlessVertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] var world_position: float4;
    [[vk::location(1)]] var view_position: float4;
    [[vk::location(2)]] var normal: float3;
    [[vk::location(3)]] var texture_coordinates: float2;
    [[vk::location(4)]] var color: float3;
    [[vk::location(5)]] var texture_index: int;
}

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(1, 0)]] var nearest_sampler: SamplerState;
[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(3, 0)]] var texture_sampler: SamplerState;
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
[[vk::binding(0, 2)]] var instance_data: StructuredBuffer<ModelInstanceData>;
[[vk::binding(0, 3)]] var textures: Texture2D[];

[[vk::constant_id(0)]] const var PASS_MODE: int;
[[vk::constant_id(1)]] const var ALPHA_TO_COVERAGE_ACTIVATED: bool;

[[shader("vertex")]]
func vs_main(input: ModelBindlessVertexInput) -> ModelBindlessVertexOutput {
    let instance = instance_data[input.instance_index];

    let world_position = mul(instance.world, float4(input.position, 1.0));
    let wind_position = world_position + float4(global_uniforms.animation_timer);
    let offset = float4(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * input.wind_affinity;
    let final_world_position = world_position + offset;

    var output: ModelBindlessVertexOutput;
    output.position = mul(global_uniforms.view_projection, final_world_position);
    output.view_position = mul(global_uniforms.view, final_world_position);
    output.world_position = final_world_position;
    output.normal = normalize(mul(instance.inv_world, float4(input.normal, 0.0)).xyz);
    output.texture_coordinates = input.texture_coordinates;
    output.color = input.color;
    output.texture_index = input.texture_index;
    return output;
}

[[shader("pixel")]]
func opaque_main(input: ModelBindlessVertexOutput) -> float4 {
    return fragment(input);
}

[[shader("pixel")]]
func transparent_main(input: ModelBindlessVertexOutput) -> WboitOutput {
    let fragment_color = fragment(input);

    let weight = clamp(pow(min(1.0, fragment_color.a * 10.0) + 0.01, 3.0) * 1e8 * pow(input.view_position.z * 0.9, 3.0), 1e-2, 3e3);

    var output: WboitOutput;
    output.accumulation = fragment_color * weight;
    output.revealage = fragment_color.a;
    return output;
}

[ForceInline]
func fragment(input: ModelBindlessVertexOutput) -> float4 {
    var diffuse_color: float4;
    var alpha_channel: float;

    if (ALPHA_TO_COVERAGE_ACTIVATED) {
        diffuse_color = textures[input.texture_index].Sample(texture_sampler, input.texture_coordinates);
        alpha_channel = diffuse_color.a;
    } else {
        diffuse_color = textures[input.texture_index].Sample(texture_sampler, input.texture_coordinates);
        alpha_channel = textures[input.texture_index].SampleLevel(nearest_sampler, input.texture_coordinates, 0.0).a;
    }

    var fragment_color = float4(0.0);

    if (PASS_MODE == 0 && alpha_channel == 0.0) {
        discard;
    } else if (PASS_MODE == 1 && (alpha_channel == 0.0 || alpha_channel <= (1.0 - MODEL_OPAQUE_EPSILON))) {
        discard;
    } else if (PASS_MODE == 2 && (alpha_channel == 0.0 || alpha_channel > (1.0 - MODEL_OPAQUE_EPSILON))) {
        discard;
    } else {
        let pixel_position = uint2(floor(input.position.xy));
        let tile_x = pixel_position.x / TILE_SIZE;
        let tile_y = pixel_position.y / TILE_SIZE;
        let tile_count_x = (global_uniforms.forward_size.x + TILE_SIZE - 1) / TILE_SIZE;
        let tile_index = tile_y * tile_count_x + tile_x;

        let light_count = light_count_texture.Load(int3(tile_x, tile_y, 0)).r;

        if (ALPHA_TO_COVERAGE_ACTIVATED) {
            uint width, height;
            textures[input.texture_index].GetDimensions(width, height);
            let texture_size = float2(width, height);
            let coverage = saturate(alpha_channel * (1.0 + max(0.0, calculate_mip_level(input.texture_coordinates * texture_size)) * MIP_SCALE));
            alpha_channel = saturate((coverage - ALPHA_CUTOFF) / max(fwidth(coverage), 0.0001) + 0.5);
        } else if (alpha_channel == 0.0) {
            discard;
        }

        let normal = normalize(input.normal);

        var ambient_light_contribution = global_uniforms.ambient_color.rgb;

        let light_direction = normalize(-directional_light.direction.xyz);
        let light_percent = max(dot(light_direction, normal), 0.0);

        let linear_view_z = depth::nonLinearToLinear(input.position.z);

        var partition_index: uint = 0;
        [unroll]
        for (var i: uint = 0; i < (PARTITION_COUNT - 1); i++) {
            if (linear_view_z >= directional_light_partitions[i].interval_end) {
                partition_index++;
            }
        }

        let offset_scales = get_shadow_offsets(normal, light_direction);
        let displacement_N = offset_scales.x * normal * SHADOW_OFFSET_NORMAL;
        let displacement_L = offset_scales.y * light_direction * SHADOW_OFFSET_LIGHT;
        let biased_world_position = input.world_position + float4(displacement_N + displacement_L, 0.0);

        let depth_bias = global_uniforms.use_sdsm ? SDSM_CONSTANT_DEPTH_BIAS : PPSM_CONSTANT_DEPTH_BIAS;

        let shadow_position = mul(directional_light_partitions[partition_index].view_projection, biased_world_position);
        var shadow_coords = shadow_position.xyz / shadow_position.w;
        shadow_coords = float3(
            coordinate_space::clip_to_screen_space(shadow_coords.xy),
            max(shadow_coords.z, 0.0) + depth_bias
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
                uint width, height, depth;
                shadow_maps.GetDimensions(width, height, depth);
                let shadow_map_dimensions = uint2(width, height);
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

        var point_light_contribution = float3(0.0);
        for (var index = 0; index < light_count; index++) {
            let light_index = tile_light_indices[tile_index].indices[index];
            let light = point_lights[light_index];
            let light_direction = normalize(input.world_position.xyz - light.position.xyz);
            let light_percent = max(dot(light_direction, normal), 0.0);
            let light_distance = length(light.position.xyz - input.world_position.xyz);
            var visibility = 1.0;

            if (light.texture_index != 0) {
                let bias = 1.2;
                let distance_to_light = depth::linearToNonLinear(light_distance - bias);

                let closest_distance = point_shadow_maps.Sample(
                    linear_sampler,
                    float4(light_direction.xyz, float(light.texture_index - 1))
                );

                visibility = float(distance_to_light > closest_distance);
            }

            let intensity = 10.0;
            let attenuation = calculate_attenuation(light_distance, light.range);
            point_light_contribution += (light.color.rgb * intensity) * light_percent * attenuation * visibility;
        }

        let base_color = diffuse_color.rgb * input.color;
        let light_contributions = saturate(ambient_light_contribution + directional_light_contribution + point_light_contribution);
        var color = base_color.rgb * light_contributions;

        if (global_uniforms.enhanced_lighting == 0) {
            color = color_balance(color, -0.01, 0.0, 0.0);
        }

        fragment_color = float4(color, diffuse_color.a);
    }

    return fragment_color;
}
