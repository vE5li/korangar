#language slang 2026

module postprocessing;

import slug;

public struct PostprocessingVertexInput {
    public uint vertex_index : SV_VulkanVertexID;
    public uint instance_index : SV_VulkanInstanceID;
}

public struct PostprocessingVertexOutput {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var texture_coordinates: float2;
    [[vk::location(1)]] public var instance_index: uint;
}

public struct DebugVertexOutput {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var color: float4;
}

public struct DebugBufferVertexOutput {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var fragment_position: float2;
}

public struct EffectInstanceData {
    public var top_left: float2;
    public var bottom_left: float2;
    public var top_right: float2;
    public var bottom_right: float2;
    public var texture_top_left: float2;
    public var texture_bottom_left: float2;
    public var texture_top_right: float2;
    public var texture_bottom_right: float2;
    public var color0: float2;
    public var color1: float2;
    public var texture_index: int;
    public var padding: uint;
}

public struct RectangleInstanceData {
    public var color: float4;
    public var screen_position: float2;
    public var screen_size: float2;
    public var texture_position: float2;
    public var texture_size: float2;
    public var rectangle_type: uint;
    public var texture_index: int;
}

public struct DebugUniforms {
    public var show_picker_buffer: uint;
    public var show_directional_shadow_map: uint;
    public var show_point_shadow_map: uint;
    public var show_light_culling_count_buffer: uint;
    public var show_sdsm_partitions: uint;
}

public static const var EDGE_VALUE: float = 0.5;
public static const var PXRANGE: float = 6.0;
public static const var SHADOW_SPREAD: float = 0.45;
// The radius of the dark shadow around text in em units. Must match
// `TEXT_SHADOW_RADIUS` of the font loader.
public static const var TEXT_SHADOW_RADIUS: float = 0.08;
public static const var TILE_SIZE: uint = 16;

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  w
// 0             0  0  0  0
// 1             1  0  1  0
// 2             1 -1  1  1
// 3             1 -1  1  1
// 4             0 -1  0  1
// 5             0  0  0  0
//
// (x,y) are the vertex position
// (z,w) are the UV coordinates
[ForceInline]
public func rectangle_vertex_data(vertex_index: uint) -> float4 {
    let index = 1u << vertex_index;
    let x = float((index & 0xEu) != 0u);
    let y = float((index & 0x1Cu) != 0u);
    return float4(x, -y, x, y);
}

[ForceInline]
public func effect_position_data(instance: EffectInstanceData, vertex_index: uint) -> float4 {
    switch (vertex_index) {
        case 0:
            return float4(instance.top_left, instance.texture_top_left);
        case 1:
            return float4(instance.bottom_left, instance.texture_bottom_left);
        case 2:
            return float4(instance.top_right, instance.texture_top_right);
        case 3:
            return float4(instance.top_right, instance.texture_top_right);
        case 4:
            return float4(instance.bottom_left, instance.texture_bottom_left);
        default:
            return float4(instance.bottom_right, instance.texture_bottom_right);
    }
}

[ForceInline]
public func calculate_shadowed_sdf(
    distance: float,
    texture_size: float2,
    texture_coordinates: float2,
    color: float4
) -> float4 {
    let unit_range = float2(PXRANGE) / texture_size;
    let screen_texture_size = float2(1.0) / fwidth(texture_coordinates);
    let screen_px_range = max(EDGE_VALUE * dot(unit_range, screen_texture_size), 1.0);

    let shadow_distance = distance + SHADOW_SPREAD;
    let shadow_alpha = saturate(shadow_distance * screen_px_range);
    let shadow_color = float4(0.0, 0.0, 0.0, shadow_alpha);

    let text_alpha = saturate(distance * screen_px_range + EDGE_VALUE);
    let text_color = color * text_alpha;

    return lerp(shadow_color, text_color, text_alpha);
}

// Renders a glyph with a dark shadow around it. The shadow covers every pixel
// that lies within the shadow radius of the glyph along one of eight directions.
public func calculate_shadowed_text(
    curves: StructuredBuffer<float4>,
    bands: StructuredBuffer<uint>,
    glyph: SlugGlyph,
    render_coordinate: float2,
    color: float4
) -> float4 {
    static const var DIAGONAL: float = 0.70710678;
    static const float2 DIRECTIONS[8] = {
        float2(1.0, 0.0),
        float2(DIAGONAL, DIAGONAL),
        float2(0.0, 1.0),
        float2(-DIAGONAL, DIAGONAL),
        float2(-1.0, 0.0),
        float2(-DIAGONAL, -DIAGONAL),
        float2(0.0, -1.0),
        float2(DIAGONAL, -DIAGONAL),
    };

    let pixels_per_em = slug_pixels_per_em(render_coordinate);
    let text_alpha = slug_coverage(curves, bands, glyph, render_coordinate, pixels_per_em);

    var shadow_alpha = text_alpha;

    for (var index = 0; index < 8; index++) {
        if (shadow_alpha >= 1.0) {
            break;
        }

        let offset = DIRECTIONS[index] * TEXT_SHADOW_RADIUS;
        shadow_alpha = max(shadow_alpha, slug_coverage(curves, bands, glyph, render_coordinate + offset, pixels_per_em));
    }

    let shadow_color = float4(0.0, 0.0, 0.0, shadow_alpha);
    let text_color = color * text_alpha;

    return lerp(shadow_color, text_color, text_alpha);
}

[ForceInline]
public func screen_to_clip_space(screen_coords: float2) -> float2 {
    let x = (screen_coords.x * 2.0) - 1.0;
    let y = -(screen_coords.y * 2.0) + 1.0;
    return float2(x, y);
}

[ForceInline]
public func clip_to_uv(clip_space_position: float2) -> float2 {
    return float2((clip_space_position.x + 1.0) * 0.5, (1.0 - clip_space_position.y) * 0.5);
}

// Debug AABB structures
public struct DebugAABBInstanceData {
    public var world: float4x4;
    public var color: float4;
}



// Debug rectangle structures
public struct DebugRectangleInstanceData {
    public var world: float4x4;
    public var color: float4;
}

public struct DebugRectangleVertexInput {
    [[vk::location(0)]] public var position: float3;
    public uint instance_index : SV_VulkanInstanceID;
}
