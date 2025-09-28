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
public static const var CONSTANT_DEPTH_BIAS: float = 0.0001;
public static const var EPSILON: float = 1.0e-7;
public static const var BANDLIMITED_PI_HALF: float = 0.5 * 3.14159265359;

public static const var SAMPLE_POINTS_8: float2[8] = {
    float2(0.125, -0.375),
    float2(-0.125, 0.375),
    float2(0.625, 0.125),
    float2(-0.375, -0.625),
    float2(-0.625, 0.625),
    float2(-0.875, -0.125),
    float2(0.375, 0.875),
    float2(0.875, -0.875)
};

public static const var SAMPLE_POINTS_16: float2[16] = {
    float2(-0.875, -0.875),
    float2(-0.750, -0.125),
    float2(-0.625, 0.625),
    float2(-0.500, -0.375),
    float2(-0.375, 0.875),
    float2(-0.250, 0.125),
    float2(-0.125, -0.625),
    float2(0.000, 0.375),
    float2(0.125, -0.750),
    float2(0.250, 0.500),
    float2(0.375, -0.250),
    float2(0.500, 0.750),
    float2(0.625, 0.000),
    float2(0.750, -0.500),
    float2(0.875, 0.250),
    float2(1.000, -1.000)
};

public static const var SAMPLE_POINTS_32: float2[32] = {
    float2(0.06407013, 0.05409927),
    float2(0.7366577, 0.5789394),
    float2(-0.6270542, -0.5320278),
    float2(-0.4096107, 0.8411095),
    float2(0.6849564, -0.4990818),
    float2(-0.874181, -0.04579735),
    float2(0.9989998, 0.0009880066),
    float2(-0.004920578, -0.9151649),
    float2(0.1805763, 0.9747483),
    float2(-0.2138451, 0.2635818),
    float2(0.109845, 0.3884785),
    float2(0.06876755, -0.3581074),
    float2(0.374073, -0.7661266),
    float2(0.3079132, -0.1216763),
    float2(-0.3794335, -0.8271583),
    float2(-0.203878, -0.07715034),
    float2(0.5912697, 0.1469799),
    float2(-0.88069, 0.3031784),
    float2(0.5040108, 0.8283722),
    float2(-0.5844124, 0.5494877),
    float2(0.6017799, -0.1726654),
    float2(-0.5554981, 0.1559997),
    float2(-0.3016369, -0.3900928),
    float2(-0.5550632, -0.1723762),
    float2(0.925029, 0.2995041),
    float2(-0.2473137, 0.5538505),
    float2(0.9183037, -0.2862392),
    float2(0.2469421, 0.6718712),
    float2(0.3916397, -0.4328209),
    float2(-0.03576927, -0.6220032),
    float2(-0.04661255, 0.7995201),
    float2(0.4402924, 0.3640312)
};

public static const var SAMPLE_POINTS_64: float2[64] = {
    float2(-0.5119625, -0.4827938),
    float2(-0.2171264, -0.4768726),
    float2(-0.7552931, -0.2426507),
    float2(-0.7136765, -0.4496614),
    float2(-0.5938849, -0.6895654),
    float2(-0.3148003, -0.7047654),
    float2(-0.42215, -0.2024607),
    float2(-0.9466816, -0.2014508),
    float2(-0.8409063, -0.03465778),
    float2(-0.6517572, -0.07476326),
    float2(-0.1041822, -0.02521214),
    float2(-0.3042712, -0.02195431),
    float2(-0.5082307, 0.1079806),
    float2(-0.08429877, -0.2316298),
    float2(-0.9879128, 0.1113683),
    float2(-0.3859636, 0.3363545),
    float2(-0.1925334, 0.1787288),
    float2(0.003256182, 0.138135),
    float2(-0.8706837, 0.3010679),
    float2(-0.6982038, 0.1904326),
    float2(0.1975043, 0.2221317),
    float2(0.1507788, 0.4204168),
    float2(0.3514056, 0.09865579),
    float2(0.1558783, -0.08460935),
    float2(-0.0684978, 0.4461993),
    float2(0.3780522, 0.3478679),
    float2(0.3956799, -0.1469177),
    float2(0.5838975, 0.1054943),
    float2(0.6155105, 0.3245716),
    float2(0.3928624, -0.4417621),
    float2(0.1749884, -0.4202175),
    float2(0.6813727, -0.2424808),
    float2(-0.6707711, 0.4912741),
    float2(0.0005130528, -0.8058334),
    float2(0.02703013, -0.6010728),
    float2(-0.1658188, -0.9695674),
    float2(0.4060591, -0.7100726),
    float2(0.7713396, -0.4713659),
    float2(0.573212, -0.51544),
    float2(-0.3448896, -0.9046497),
    float2(0.1268544, -0.9874692),
    float2(0.7418533, -0.6667366),
    float2(0.3492522, 0.5924662),
    float2(0.5679897, 0.5343465),
    float2(0.5663417, 0.7708698),
    float2(0.7375497, 0.6691415),
    float2(0.2271994, -0.6163502),
    float2(0.2312844, 0.8725659),
    float2(0.4216993, 0.9002838),
    float2(0.4262091, -0.9013284),
    float2(0.2001408, -0.808381),
    float2(0.149394, 0.6650763),
    float2(-0.09640376, 0.9843736),
    float2(0.7682328, -0.07273844),
    float2(0.04146584, 0.8313184),
    float2(0.9705266, -0.1143304),
    float2(0.9670017, 0.1293385),
    float2(0.9015037, -0.3306949),
    float2(-0.5085648, 0.7534177),
    float2(0.9055501, 0.3758393),
    float2(0.7599946, 0.1809109),
    float2(-0.2483695, 0.7942952),
    float2(-0.4241052, 0.5581087),
    float2(-0.1020106, 0.6724468)
};

public func get_shadow_offsets(normal: float3, light_direction: float3) -> float2 {
    let cos_alpha = saturate(dot(normal, light_direction));
    let sin_alpha = sqrt(1.0 - (cos_alpha * cos_alpha));
    let tan_alpha = sin_alpha / max(0.001, cos_alpha);
    return float2(sin_alpha, min(2.0, tan_alpha));
}

public func color_balance(color: float3, cyan_red: float, magenta_green: float, yellow_blue: float) -> float3 {
    let rgb = color;
    let adjusted = float3(
        rgb.r + cyan_red,
        rgb.g + magenta_green,
        rgb.b + yellow_blue
    );
    return clamp(adjusted, float3(0.0), float3(1.0));
}

public func calculate_attenuation(distance: float, range: float) -> float {
    let effective_distance = min(distance, range);
    let normalized_distance = effective_distance / (range + EPSILON);
    let attenuation = saturate(1.0 - normalized_distance * normalized_distance);
    return attenuation * attenuation;
}

public func calculate_mip_level(texture_coordinate: float2) -> float {
    let dx = ddx(texture_coordinate);
    let dy = ddy(texture_coordinate);
    let delta_max_squared = max(dot(dx, dx), dot(dy, dy));
    return max(0.0, 0.5 * log2(delta_max_squared));
}

public func get_pcf_shadow(
    partition_index: uint,
    shadow_coords: float3,
    shadow_map_dimensions: uint2,
    shadow_maps: DepthTexture2DArray,
    shadow_map_sampler: SamplerComparisonState
) -> float {
    var gaussian_offset: int;
    switch (shadow_map_dimensions.x) {
        case 4096:
            gaussian_offset = 4;
            break;
        case 3072:
            gaussian_offset = 3;
            break;
        default:
            gaussian_offset = 2;
            break;
    }

    let texel_size = float2(1.0) / float2(shadow_map_dimensions);
    let depth = shadow_coords.z;
    var shadow: float = 0.0;
    var total_weight: float = 0.0;

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

public func get_pcf_pcss_shadow(
    partition_index: uint,
    shadow_coords: float3,
    global_uniforms: GlobalUniforms,
    directional_light_partitions: StructuredBuffer<DirectionalLightPartition>,
    shadow_maps: DepthTexture2DArray,
    shadow_map_sampler: SamplerComparisonState,
    linear_sampler: SamplerState
) -> float {
    uint width, height, depth;
    shadow_maps.GetDimensions(width, height, depth);
    let shadow_map_size = float2(width, height);
    let world_space_texel_size = directional_light_partitions[partition_index].world_space_texel_size;
    let light_size_uv = LIGHT_WORLD_SIZE / (world_space_texel_size * shadow_map_size.x);

    let blocker = find_blocker(partition_index, shadow_coords.xy, shadow_coords.z, light_size_uv, global_uniforms, shadow_maps, linear_sampler);
    let average_blocker_depth = blocker.x;
    let blocker_count = blocker.y;

    if (blocker_count == 0.0) {
        return 1.0;
    }

    let receiver_distance = shadow_coords.z;
    let blocker_distance = average_blocker_depth;
    let depth_difference = abs(receiver_distance - blocker_distance);
    let penumbra = depth_difference * light_size_uv * 2.0;

    return pcf_filter(partition_index, shadow_coords, penumbra, global_uniforms, shadow_maps, shadow_map_sampler);
}

public func find_blocker(
    partition_index: uint,
    uv: float2,
    receiver_depth: float,
    light_size_uv: float,
    global_uniforms: GlobalUniforms,
    shadow_maps: DepthTexture2DArray,
    linear_sampler: SamplerState
) -> float2 {
    var blocker_sum = 0.0;
    var blocker_count = 0.0;

    switch (global_uniforms.shadow_quality) {
        default: {
            for (var i = 0; i < 16; i++) {
                let offset = SAMPLE_POINTS_16[i] * light_size_uv;
                let shadow_depth = shadow_maps.Sample(linear_sampler, float3(uv + offset, partition_index));

                if(receiver_depth < shadow_depth) {
                    blocker_sum += shadow_depth;
                    blocker_count += 1.0;
                }
            }
            break;
        }
        case 4: {
            for (var i = 0; i < 32; i++) {
                let offset = SAMPLE_POINTS_32[i] * light_size_uv;
                let shadow_depth = shadow_maps.Sample(linear_sampler, float3(uv + offset, partition_index));

                if(receiver_depth < shadow_depth) {
                    blocker_sum += shadow_depth;
                    blocker_count += 1.0;
                }
            }
            break;
        }
        case 5: {
            for (var i = 0; i < 64; i++) {
                let offset = SAMPLE_POINTS_64[i] * light_size_uv;
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

public func pcf_filter(
    partition_index: uint,
    shadow_coords: float3,
    filter_radius_uv: float,
    global_uniforms: GlobalUniforms,
    shadow_maps: DepthTexture2DArray,
    shadow_map_sampler: SamplerComparisonState
) -> float {
    var visibility = 0.0;
    let clamped_radius = min(filter_radius_uv, 0.05);

    switch (global_uniforms.shadow_quality) {
        default: {
            for (var i = 0; i < 8; i++) {
                let offset = SAMPLE_POINTS_8[i] * clamped_radius;
                visibility += shadow_maps.SampleCmp(shadow_map_sampler, float3(shadow_coords.xy + offset, partition_index), shadow_coords.z);
            }
            return visibility / float(8);
        }
        case 3: {
            for (var i = 0; i < 16; i++) {
                let offset = SAMPLE_POINTS_16[i] * clamped_radius;
                visibility += shadow_maps.SampleCmp(shadow_map_sampler, float3(shadow_coords.xy + offset, partition_index), shadow_coords.z);
            }
            return visibility / float(16);
        }
        case 4: {
            for (var i = 0; i < 32; i++) {
                let offset = SAMPLE_POINTS_32[i] * clamped_radius;
                visibility += shadow_maps.SampleCmp(shadow_map_sampler, float3(shadow_coords.xy + offset, partition_index), shadow_coords.z);
            }
            return visibility / float(32);
        }
        case 5: {
            for (var i = 0; i < 64; i++) {
                let offset = SAMPLE_POINTS_64[i] * clamped_radius;
                visibility += shadow_maps.SampleCmp(shadow_map_sampler, float3(shadow_coords.xy + offset, partition_index), shadow_coords.z);
            }
            return visibility / float(64);
        }
    }
}

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

public func sample_bandlimited_pixel(texture: Texture2D, linear_sampler: SamplerState, uv: float2, info: BandlimitedPixelInfo, lod_bias: float) -> float4 {
    var color = texture.SampleLevel(linear_sampler, uv, lod_bias);

    if (info.length > 0.0) {
        let bandlimited = texture.SampleLevel(linear_sampler, info.uv0, 0.0);
        color = lerp(color, bandlimited, info.length);
    }

    return color;
}

public func sample_bandlimited_pixel_bindless(textures: Texture2D[], texture_index: int, linear_sampler: SamplerState, uv: float2, info: BandlimitedPixelInfo, lod_bias: float) -> float4 {
    var color = textures[texture_index].SampleLevel(linear_sampler, uv, lod_bias);

    if (info.length > 0.0) {
        let bandlimited = textures[texture_index].SampleLevel(linear_sampler, info.uv0, 0.0);
        color = lerp(color, bandlimited, info.length);
    }

    return color;
}
