#language slang 2026

module sdsm;

import globals;
import transform;

public static const var PARTITION_COUNT: uint = 3;
public static const var VIRTUAL_FAR_PLANE: float = 1000.0;
public static const var ATOMIC_FLOAT_OFFSET: float = 1.0;

public static const var REDUCE_BOUNDS_BLOCK_X: uint = 16;
public static const var REDUCE_BOUNDS_BLOCK_Y: uint = 8;
public static const var REDUCE_BOUNDS_BLOCK_SIZE: uint = REDUCE_BOUNDS_BLOCK_X * REDUCE_BOUNDS_BLOCK_Y;
public static const var REDUCE_BOUNDS_SHARED_MEMORY_ARRAY_SIZE: uint = PARTITION_COUNT * REDUCE_BOUNDS_BLOCK_SIZE;
public static const var REDUCE_TILE_DIM: uint = 64;

public static const var REDUCE_ZBOUNDS_BLOCK_DIM: uint = 16;
public static const var REDUCE_ZBOUNDS_BLOCK_SIZE: uint = REDUCE_ZBOUNDS_BLOCK_DIM * REDUCE_ZBOUNDS_BLOCK_DIM;

public struct Partition {
    public var extents: float4;
    public var center: float4;
    public var interval_begin: float;
    public var interval_end: float;
}

public struct Interval {
    public var begin: Atomic<uint>;
    public var end: Atomic<uint>;
}

public struct Bounds {
    public var min_coord_x: Atomic<uint>;
    public var min_coord_y: Atomic<uint>;
    public var min_coord_z: Atomic<uint>;
    public var max_coord_x: Atomic<uint>;
    public var max_coord_y: Atomic<uint>;
    public var max_coord_z: Atomic<uint>;
}

public struct BoundsFloat {
    public var min_coord: float3;
    public var max_coord: float3;
}

public struct DirectionalLightUniforms {
    public var view_projection: float4x4;
    public var color: float4;
    public var direction: float4;
}

// PSSM partitioning scheme: Blend between logarithmic and uniform distribution
// GPU Gems 3: Parallel-Split Shadow Maps on Programmable GPUs (2008)
[ForceInline]
public func pssm_partition_from_range(partition_index: uint, min_z: float, max_z: float) -> float {
    static const var BLEND_FACTOR: float = 0.5;
    let ratio = max_z / min_z;
    let power = float(partition_index) / float(PARTITION_COUNT);
    let log_split = min_z * pow(ratio, power);
    let uniform_split = min_z + (max_z - min_z) * (float(partition_index) / float(PARTITION_COUNT));
    return lerp(uniform_split, log_split, BLEND_FACTOR);
}

// Transform world position to light space.
[ForceInline]
public func transform_to_light_space(world_pos: float3, light_view_projection: float4x4) -> float3 {
    let light_clip = mul(light_view_projection, float4(world_pos, 1.0));
    return light_clip.xyz;
}

// Reconstruct world position from depth buffer.
[ForceInline]
public func reconstruct_position_from_depth(coords: uint2, depth: float, depth_dim: uint2, global_uniforms: GlobalUniforms) -> float3 {
    // Convert screen coordinates to NDC.
    let uv = float2(coords) / float2(depth_dim);
    let ndc_xy = uv * 2.0 - 1.0;
    let ndc = float4(ndc_xy.x, -ndc_xy.y, depth, 1.0);

    // Transform to view space.
    let view_pos = mul(global_uniforms.inverse_projection, ndc);
    let view_pos_3d = view_pos.xyz / view_pos.w;

    // Transform to world space.
    let world_pos = mul(global_uniforms.inverse_view, float4(view_pos_3d, 1.0));

    return world_pos.xyz;
}

// Read bounds as float from atomic bounds.
[ForceInline]
public func read_bounds_as_float(partition_index: uint, bounds_data: RWStructuredBuffer<Bounds>) -> BoundsFloat {
    let min_coord_offset = float3(
        asfloat(bounds_data[partition_index].min_coord_x.load(MemoryOrder::Relaxed)),
        asfloat(bounds_data[partition_index].min_coord_y.load(MemoryOrder::Relaxed)),
        asfloat(bounds_data[partition_index].min_coord_z.load(MemoryOrder::Relaxed))
    );

    let max_coord_offset = float3(
        asfloat(bounds_data[partition_index].max_coord_x.load(MemoryOrder::Relaxed)),
        asfloat(bounds_data[partition_index].max_coord_y.load(MemoryOrder::Relaxed)),
        asfloat(bounds_data[partition_index].max_coord_z.load(MemoryOrder::Relaxed))
    );

    // Subtract the offset to return to the original range.
    let min_coord = min_coord_offset - ATOMIC_FLOAT_OFFSET;
    let max_coord = max_coord_offset - ATOMIC_FLOAT_OFFSET;

    return BoundsFloat(min_coord, max_coord);
}

// Create empty bounds for initialization.
[ForceInline]
public func empty_bounds_float() -> BoundsFloat {
    return BoundsFloat(
        float3(1e30, 1e30, 1e30), // Very large value instead of f32::MAX
        float3(0.0, 0.0, 0.0)
    );
}
