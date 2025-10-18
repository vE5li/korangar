#language slang 2026

module forward;

import depth_texture;
import globals;
import matrix;
import sdsm;
import transform;

public struct DirectionalLightPartition {
    public var view_projection: float4x4;
    public var interval_end: float;
    public var world_space_texel_size: float;
    public var near_plane: float;
    public var far_plane: float;
}

public struct PointLight {
    public var position: float4;
    public var color: float4;
    public var range: float;
    public var texture_index: int;
}

public struct KernelUniforms {
    // Shadow sampling patterns (Poisson disk samples)
    // Packed as vec4 for std140 layout: vec4(point1.xy, point2.xy)
    public var sample_points_8: float4[4];
    public var sample_points_16: float4[8];
    public var sample_points_32: float4[16];
    public var sample_points_64: float4[32];
}

public struct ModelInstanceData {
    public var world: float4x4;
    public var inv_world: float4x4;
}

public struct ForwardEntityInstanceData {
    public var world: float4x4;
    public var frame_part_transform: float4x4;
    public var texture_position: float2;
    public var texture_size: float2;
    public var color: float4;
    public var frame_size: float2;
    public var extra_depth_offset: float;
    public var depth_offset: float;
    public var curvature: float;
    public var mirror: uint;
    public var texture_index: int;
};

public struct TileLightIndices {
    public var indices: uint[256];
}

public struct WboitOutput {
    [[vk::location(1)]] public var accumulation: float4;
    [[vk::location(2)]] public var revealage: float;
}

public struct ForwardEntityFragmentOutput {
    public float depth : SV_Depth;
    [[vk::location(0)]] public var color: float4;
};

public struct ForwardEntityWboitOutput {
    public float depth : SV_Depth;
    [[vk::location(1)]] public var accumulation: float4;
    [[vk::location(2)]] public var revealage: float;
};

public struct BandlimitedPixelInfo {
    public var uv0: float2;
    public var length: float;
}

public static const var MIP_SCALE: float = 0.25;
public static const var ALPHA_CUTOFF: float = 0.4;
public static const var TILE_SIZE: uint = 16;
public static const var MODEL_OPAQUE_EPSILON: float = 0.01;
public static const var ENTITY_OPAQUE_EPSILON: float = 0.2;
public static const var LIGHT_WORLD_SIZE: float = 5.0;
public static const var SHADOW_OFFSET_NORMAL: float = -0.2;
public static const var SHADOW_OFFSET_LIGHT: float = -0.2;
public static const var SDSM_CONSTANT_DEPTH_BIAS: float = 0.0001;
public static const var PPSM_CONSTANT_DEPTH_BIAS: float = 0.0005;
public static const var EPSILON: float = 1.0e-7;
public static const var BANDLIMITED_PI_HALF: float = 0.5 * 3.14159265359;

[ForceInline]
public func get_shadow_offsets(normal: float3, light_direction: float3) -> float2 {
    let cos_alpha = saturate(dot(normal, light_direction));
    let sin_alpha = sqrt(1.0 - (cos_alpha * cos_alpha));
    let tan_alpha = sin_alpha / max(0.001, cos_alpha);
    return float2(sin_alpha, min(2.0, tan_alpha));
}

[ForceInline]
public func color_balance(color: float3, cyan_red: float, magenta_green: float, yellow_blue: float) -> float3 {
    let rgb = color;
    let adjusted = float3(
        rgb.r + cyan_red,
        rgb.g + magenta_green,
        rgb.b + yellow_blue
    );
    return clamp(adjusted, float3(0.0), float3(1.0));
}

[ForceInline]
public func calculate_attenuation(distance: float, range: float) -> float {
    let effective_distance = min(distance, range);
    let normalized_distance = effective_distance / (range + EPSILON);
    let attenuation = saturate(1.0 - normalized_distance * normalized_distance);
    return attenuation * attenuation;
}

[ForceInline]
public func calculate_mip_level(texture_coordinate: float2) -> float {
    let dx = ddx(texture_coordinate);
    let dy = ddy(texture_coordinate);
    let delta_max_squared = max(dot(dx, dx), dot(dy, dy));
    return max(0.0, 0.5 * log2(delta_max_squared));
}

[ForceInline]
public func get_pcf_shadow(
    partition_index: uint,
    shadow_coords: float3,
    shadow_map_dimensions: uint2,
    shadow_detail: uint,
    shadow_maps: DepthTexture2DArray,
    shadow_map_sampler: SamplerComparisonState
) -> float {
    let texel_size = float2(1.0) / float2(shadow_map_dimensions);
    let depth = shadow_coords.z;
    var shadow: float = 0.0;
    var total_weight: float = 0.0;

    switch (shadow_detail) {
        case 1: {
            let samples = shadow_maps.GatherCmp(shadow_map_sampler, float3(shadow_coords.xy, partition_index), depth);
            return (samples.x + samples.y + samples.z + samples.w) / 4.0;
        }
        default: {
            var gaussian_offset: int;
            switch (shadow_detail) {
                case 2:
                    gaussian_offset = 2;
                    break;
                case 3:
                    gaussian_offset = 3;
                    break;
                default: // case 4
                    gaussian_offset = 4;
                    break;
            }

            let gaussian_offset_pow2 = float(gaussian_offset * gaussian_offset);
            let sigma_squared = gaussian_offset_pow2 * 0.25;
            let weight_factor = 1.0 / (2.0 * sigma_squared);

            for (var y: int = -gaussian_offset; y <= gaussian_offset; y += 2) {
                for (var x: int = -gaussian_offset; x <= gaussian_offset; x += 2) {
                    let offset = float2(float(x), float(y)) * texel_size;
                    let distance_squared = float(x * x + y * y);
                    let weight = exp(-distance_squared * weight_factor);

                    let samples = shadow_maps.GatherCmp(shadow_map_sampler, float3(shadow_coords.xy + offset, partition_index), depth);
                    shadow += (samples.x + samples.y + samples.z + samples.w) * weight;
                    total_weight += 4.0 * weight;
                }
            }

            return shadow / total_weight;
        }
    }
}

[ForceInline]
public func get_pcf_pcss_shadow(
    partition_index: uint,
    shadow_coords: float3,
    global_uniforms: GlobalUniforms,
    directional_light_partitions: StructuredBuffer<DirectionalLightPartition>,
    shadow_maps: DepthTexture2DArray,
    shadow_map_sampler: SamplerComparisonState,
    linear_sampler: SamplerState,
    kernel_uniforms: ConstantBuffer<KernelUniforms>
) -> float {
    uint width, height, depth;
    shadow_maps.GetDimensions(width, height, depth);
    let shadow_map_size = float2(width, height);
    let world_space_texel_size = directional_light_partitions[partition_index].world_space_texel_size;
    let light_size_uv = LIGHT_WORLD_SIZE / (world_space_texel_size * shadow_map_size.x);

    let blocker = find_blocker(partition_index, shadow_coords.xy, shadow_coords.z, light_size_uv, global_uniforms.shadow_detail, shadow_maps, linear_sampler, kernel_uniforms);
    let average_blocker_depth = blocker.x;
    let blocker_count = blocker.y;

    if (blocker_count == 0.0) {
        return 1.0;
    }

    let receiver_distance = shadow_coords.z;
    let blocker_distance = average_blocker_depth;
    let depth_difference = abs(receiver_distance - blocker_distance);
    let penumbra = depth_difference * light_size_uv * 2.0;

    return pcf_filter(partition_index, shadow_coords, penumbra, global_uniforms.shadow_detail, shadow_maps, shadow_map_sampler, kernel_uniforms);
}

[ForceInline]
public func find_blocker(
    partition_index: uint,
    uv: float2,
    receiver_depth: float,
    light_size_uv: float,
    shadow_detail: uint,
    shadow_maps: DepthTexture2DArray,
    linear_sampler: SamplerState,
    kernel_uniforms: ConstantBuffer<KernelUniforms>
) -> float2 {
    var blocker_sum = 0.0;
    var blocker_count = 0.0;

    switch (shadow_detail) {
        case 1:
        case 2: {
            [unroll]
            for (var i = 0; i < 16; i++) {
                let packed = kernel_uniforms.sample_points_16[i / 2];
                let offset = (i % 2 == 0 ? packed.xy : packed.zw) * light_size_uv;
                let shadow_depth = shadow_maps.Sample(linear_sampler, float3(uv + offset, partition_index));

                if(receiver_depth < shadow_depth) {
                    blocker_sum += shadow_depth;
                    blocker_count += 1.0;
                }
            }
            break;
        }
        case 3: {
            [unroll]
            for (var i = 0; i < 32; i++) {
                let packed = kernel_uniforms.sample_points_32[i / 2];
                let offset = (i % 2 == 0 ? packed.xy : packed.zw) * light_size_uv;
                let shadow_depth = shadow_maps.Sample(linear_sampler, float3(uv + offset, partition_index));

                if(receiver_depth < shadow_depth) {
                    blocker_sum += shadow_depth;
                    blocker_count += 1.0;
                }
            }
            break;
        }
        default: {
            [unroll]
            for (var i = 0; i < 64; i++) {
                let packed = kernel_uniforms.sample_points_64[i / 2];
                let offset = (i % 2 == 0 ? packed.xy : packed.zw) * light_size_uv;
                let shadow_depth = shadow_maps.Sample(linear_sampler, float3(uv + offset, partition_index));

                if(receiver_depth < shadow_depth) {
                    blocker_sum += shadow_depth;
                    blocker_count += 1.0;
                }
            }
            break;
        }
    }

    if(blocker_count == 0.0) {
        return float2(-1.0, 0.0);
    }

    return float2(blocker_sum / blocker_count, blocker_count);
}

[ForceInline]
public func pcf_filter(
    partition_index: uint,
    shadow_coords: float3,
    filter_radius_uv: float,
    shadow_detail: uint,
    shadow_maps: DepthTexture2DArray,
    shadow_map_sampler: SamplerComparisonState,
    kernel_uniforms: ConstantBuffer<KernelUniforms>
) -> float {
    var visibility = 0.0;
    let clamped_radius = min(filter_radius_uv, 0.05);

    switch (shadow_detail) {
        case 1: {
            [unroll]
            for (var i = 0; i < 8; i++) {
                let packed = kernel_uniforms.sample_points_8[i / 2];
                let offset = (i % 2 == 0 ? packed.xy : packed.zw) * clamped_radius;
                visibility += shadow_maps.SampleCmp(shadow_map_sampler, float3(shadow_coords.xy + offset, partition_index), shadow_coords.z);
            }
            return visibility / float(8);
        }
        case 2: {
            [unroll]
            for (var i = 0; i < 16; i++) {
                let packed = kernel_uniforms.sample_points_16[i / 2];
                let offset = (i % 2 == 0 ? packed.xy : packed.zw) * clamped_radius;
                visibility += shadow_maps.SampleCmp(shadow_map_sampler, float3(shadow_coords.xy + offset, partition_index), shadow_coords.z);
            }
            return visibility / float(16);
        }
        case 3: {
            [unroll]
            for (var i = 0; i < 32; i++) {
                let packed = kernel_uniforms.sample_points_32[i / 2];
                let offset = (i % 2 == 0 ? packed.xy : packed.zw) * clamped_radius;
                visibility += shadow_maps.SampleCmp(shadow_map_sampler, float3(shadow_coords.xy + offset, partition_index), shadow_coords.z);
            }
            return visibility / float(32);
        }
        default: {
            [unroll]
            for (var i = 0; i < 64; i++) {
                let packed = kernel_uniforms.sample_points_64[i / 2];
                let offset = (i % 2 == 0 ? packed.xy : packed.zw) * clamped_radius;
                visibility += shadow_maps.SampleCmp(shadow_map_sampler, float3(shadow_coords.xy + offset, partition_index), shadow_coords.z);
            }
            return visibility / float(64);
        }
    }
}

[ForceInline]
public func rotateY(direction: float3, angle: float) -> float3 {
    let s = sin(angle);
    let c = cos(angle);
    let rotation_matrix = float3x3(
        c, 0.0, -s,
        0.0, 1.0, 0.0,
        s, 0.0, c
    );
    return mul(rotation_matrix, direction);
}

[ForceInline]
public func compute_pixel_weights(uv: float2, texture_size: float2, inverse_texture_size: float2, extent_mod: float) -> BandlimitedPixelInfo {
    // Get derivatives in texel space.
    // Need a non-zero derivative.
    let extent = max(fwidth(uv) * texture_size * extent_mod, float2(1.0 / 256.0));

    // Get base pixel and phase, range [0, 1).
    let pixel = uv * texture_size - 0.5;
    let base_pixel = floor(pixel);
    let phase = pixel - base_pixel;

    if (any(extent > float2(1.0))) {
        // We need to just do regular minimization filtering.
        var info: BandlimitedPixelInfo;
        info.uv0 = float2(0.0);
        info.length = 0.0;
        return info;
    } else {
        // We can resolve the filter by just sampling a single 2x2 block.
        // Lerp between normal sampling at LOD 0, and bandlimited pixel filter at LOD -1.
        let shift = 0.5 + 0.5 * sin(BANDLIMITED_PI_HALF * clamp((phase - 0.5) / min(extent, float2(0.5)), float2(-1.0), float2(1.0)));
        let max_extent = max(extent.x, extent.y);
        let length = clamp(2.0 - 2.0 * max_extent, 0.0, 1.0);

        var info: BandlimitedPixelInfo;
        info.uv0 = (base_pixel + 0.5 + shift) * inverse_texture_size;
        info.length = length;
        return info;
    }
}

[ForceInline]
public func sample_bandlimited_pixel(texture: Texture2D, linear_sampler: SamplerState, uv: float2, info: BandlimitedPixelInfo, lod_bias: float) -> float4 {
    var color = texture.SampleLevel(linear_sampler, uv, lod_bias);

    if (info.length > 0.0) {
        let bandlimited = texture.SampleLevel(linear_sampler, info.uv0, 0.0);
        color = lerp(color, bandlimited, info.length);
    }

    return color;
}

[ForceInline]
public func sample_bandlimited_pixel_bindless(textures: Texture2D[], texture_index: int, linear_sampler: SamplerState, uv: float2, info: BandlimitedPixelInfo, lod_bias: float) -> float4 {
    var color = textures[texture_index].SampleLevel(linear_sampler, uv, lod_bias);

    if (info.length > 0.0) {
        let bandlimited = textures[texture_index].SampleLevel(linear_sampler, info.uv0, 0.0);
        color = lerp(color, bandlimited, info.length);
    }

    return color;
}
