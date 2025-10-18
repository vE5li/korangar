#language slang 2026

import depth_texture;
import globals;
import forward;
import directional_shadow;
import matrix;
import transform;
import sdsm;

struct ForwardEntityVertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] var world_position: float4;
    [[vk::location(1)]] var texture_coordinates: float2;
    [[vk::location(2)]] var normal: float3;
    [[vk::location(3)]] var depth_offset: float;
    [[vk::location(4)]] var curvature: float;
    [[vk::location(5)]] var original_depth_offset: float;
    [[vk::location(6)]] var original_curvature: float;
    [[vk::location(7)]] var color: float4;
};

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
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
[[vk::binding(0, 2)]] var instance_data: StructuredBuffer<ForwardEntityInstanceData>;
[[vk::binding(0, 3)]] var texture: Texture2D;

[[vk::constant_id(0)]] const var PASS_MODE: uint;

[[shader("vertex")]]
func vs_main(input: EntityVertexInput) -> ForwardEntityVertexOutput {
    let instance = instance_data[input.instance_index];
    let vertex = entity_vertex_data(input.vertex_index);
    let frame_part_vertex = mul(instance.frame_part_transform, float4(vertex.position, 1.0));
    let world_position = mul(instance.world, frame_part_vertex);

    var output: ForwardEntityVertexOutput;
    output.world_position = world_position;
    output.position = mul(global_uniforms.view_projection, world_position);
    output.texture_coordinates = instance.texture_position + vertex.texture_coordinates * instance.texture_size;

    if (instance.mirror != 0) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    let rotated = rotateY(float3(global_uniforms.view.getColumn(2).x, 0, global_uniforms.view.getColumn(2).z), vertex.position.x);
    output.normal = float3(-rotated.x, rotated.y, rotated.z);

    // Values are represented as proportions ranging from -1 to 1
    let proportion_x = instance.frame_size.x / SPRITE_MAX_SIZE_X;
    let proportion_y = instance.frame_size.y / SPRITE_MAX_SIZE_Y;

    // The depth multiplier and curvature multiplier is derived from the truth table of entity_vertex_data
    // Because we have to transform the vertex of the frame part, we can't use the depth and curvature
    // directly and are using the fact, that y / depth and x / curvature correlate to each other.
    // An offset is also added for frame parts not stay at the same depth.
    output.depth_offset = frame_part_vertex.y * proportion_y + instance.extra_depth_offset;
    output.curvature = frame_part_vertex.x * proportion_x;

    output.original_depth_offset = instance.depth_offset;
    output.original_curvature = instance.curvature;
    output.color = instance.color;
    return output;
}

[[shader("pixel")]]
func opaque_main(input: ForwardEntityVertexOutput) -> ForwardEntityFragmentOutput {
    return fragment(input);
}

[[shader("pixel")]]
func transparent_main(input: ForwardEntityVertexOutput) -> ForwardEntityWboitOutput {
    let frag = fragment(input);

    // Equation from https://casual-effects.blogspot.com/2015/03/implemented-weighted-blended-order.html
    let weight = clamp(pow(min(1.0, frag.color.a * 10.0) + 0.01, 3.0) * 1e8 * pow(frag.depth * 0.9, 3.0), 1e-2, 3e3);

    var output: ForwardEntityWboitOutput;
    output.accumulation = frag.color * weight;
    output.revealage = frag.color.a;
    output.depth = frag.depth;
    return output;
}

[ForceInline]
func fragment(input: ForwardEntityVertexOutput) -> ForwardEntityFragmentOutput {
    uint width, height;
    texture.GetDimensions(width, height);
    let texture_dimensions = float2(width, height);
    let inverse_texture_dimensions = float2(1.0) / texture_dimensions;

    let pixel_info = compute_pixel_weights(
        input.texture_coordinates,
        texture_dimensions,
        inverse_texture_dimensions,
        1.0
    );

    let diffuse_color = sample_bandlimited_pixel(
        texture,
        linear_sampler,
        input.texture_coordinates,
        pixel_info,
        0.0
    );

    var alpha_channel = diffuse_color.a;
    alpha_channel *= input.color.a;

    var output: ForwardEntityFragmentOutput;

    if (PASS_MODE == 0 && (alpha_channel == 0.0 || alpha_channel <= (1.0 - ENTITY_OPAQUE_EPSILON))) {
        // Opaque pass can early exit if invisible or transparent pixel was found.
        discard;
    } else if (PASS_MODE == 1 && (alpha_channel == 0.0 || alpha_channel > (1.0 - ENTITY_OPAQUE_EPSILON))) {
        // Transparent pass can early exit if invisible or opaque pixel was found.
        discard;
    } else {
        // Calculate which tile this fragment belongs to
        let pixel_position = uint2(floor(input.position.xy));
        let tile_x = pixel_position.x / TILE_SIZE;
        let tile_y = pixel_position.y / TILE_SIZE;
        let tile_count_x = (global_uniforms.forward_size.x + TILE_SIZE - 1) / TILE_SIZE;
        let tile_index = tile_y * tile_count_x + tile_x;

        // Get the number of lights affecting this tile
        let light_count = light_count_texture.Load(int3(tile_x, tile_y, 0)).r;

        let normal = normalize(input.normal);

        // Adjust the sprite as if it was standing upright
        let depth_offset = input.depth_offset * input.original_depth_offset;
        let curvature_offset = (0.5 - pow(input.curvature, 2.0)) * input.original_curvature;
        let view_position = mul(global_uniforms.view, input.world_position);
        let adjusted_view_position = view_position - float4(0.0, 0.0, depth_offset + curvature_offset, 0.0);
        let adjusted_world_position = mul(global_uniforms.inverse_view, adjusted_view_position);
        let clip_position = mul(global_uniforms.view_projection, adjusted_world_position);
        let frag_depth = saturate(clip_position.z / clip_position.w);

        // Ambient light
        var ambient_light_contribution = global_uniforms.ambient_color.rgb;

        // Directional light
        let light_direction = normalize(-directional_light.direction.xyz);
        var light_percent = 1.0;

        if (global_uniforms.enhanced_lighting != 0) {
            light_percent = max(dot(light_direction, normal), 0.0);
        }

        // Shadow calculation
        let linear_view_z = depth::nonLinearToLinear(input.position.z);

        var partition_index: uint = 0;
        [unroll]
        for (var i: uint = 0; i < (PARTITION_COUNT - 1); i++) {
            if (linear_view_z >= directional_light_partitions[i].interval_end) {
                partition_index++;
            }
        }

        let shadow_position = mul(directional_light_partitions[partition_index].view_projection, adjusted_world_position);
        var shadow_coords = shadow_position.xyz / shadow_position.w;
        let bias = 0.011;
        shadow_coords = float3(
            coordinate_space::clip_to_screen_space(shadow_coords.xy),
            // If the far plane of the shadow is in front of the drawn object, then it's value would be below 0.0.
            // That would create a false shadow, so we need to make sure the value is never below 0.0 so that this
            // phantom shadowing is not occurring (can happen when zooming).
            max(shadow_coords.z, 0.0) + bias
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

        let directional_light_contribution = directional_light.color.rgb * light_percent * visibility;

        // Point lights
        var point_light_contribution = float3(0.0);
        for (var index = 0; index < light_count; index++) {
            let light_index = tile_light_indices[tile_index].indices[index];
            let light = point_lights[light_index];
            let light_direction_point = normalize(adjusted_world_position.xyz - light.position.xyz);
            let light_percent_point = max(dot(light_direction_point, input.normal), 0.0);
            let light_distance = length(light.position.xyz - adjusted_world_position.xyz);
            var visibility_point = 1.0;

            if (light.texture_index != 0) {
                let point_bias = 1.2;
                let distance_to_light = depth::linearToNonLinear(light_distance - point_bias);

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

        let base_color = diffuse_color * input.color;
        let light_contributions = saturate(ambient_light_contribution + directional_light_contribution + point_light_contribution);
        var color = base_color.rgb * light_contributions;

        if (global_uniforms.enhanced_lighting == 0) {
            color = color_balance(color, -0.01, 0.0, 0.0);
        }

        output.color = float4(color, alpha_channel);
        output.depth = frag_depth;
    }

    return output;
}
