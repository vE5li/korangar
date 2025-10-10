#language slang 2026

import depth_texture;
import globals;
import sdsm;
import transform;

struct ComputeInput {
    uint3 global_invocation_id : SV_DispatchThreadID;
    uint3 local_invocation_id : SV_GroupThreadID;
    uint3 workgroup_id : SV_GroupID;
    uint local_invocation_index : SV_GroupIndex;
}

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(0, 1)]] var directional_light: ConstantBuffer<DirectionalLightUniforms>;
[[vk::binding(1, 1)]] var depth_texture: DepthTexture2DMS;
[[vk::binding(2, 1)]] var partition_data: RWStructuredBuffer<Partition>;
[[vk::binding(4, 1)]] var bounds_data: RWStructuredBuffer<Bounds>;

groupshared var sbounds_min: float3[REDUCE_BOUNDS_SHARED_MEMORY_ARRAY_SIZE];
groupshared var sbounds_max: float3[REDUCE_BOUNDS_SHARED_MEMORY_ARRAY_SIZE];

[[shader("compute")]]
[[numthreads(REDUCE_BOUNDS_BLOCK_X, REDUCE_BOUNDS_BLOCK_Y, 1)]]
func main(ComputeInput input) {
    var depth_dim: uint2;
    var sample_count: uint;
    depth_texture.GetDimensions(depth_dim.x, depth_dim.y, sample_count);

    var bounds_reduce: BoundsFloat[PARTITION_COUNT];
    [unroll]
    for (uint partition_index = 0; partition_index < PARTITION_COUNT; partition_index++) {
        bounds_reduce[partition_index] = empty_bounds_float();
    }

    let near_z = partition_data[0].interval_begin;
    let far_z = partition_data[PARTITION_COUNT - 1].interval_end;

    let tile_start = input.workgroup_id.xy * REDUCE_TILE_DIM + input.local_invocation_id.xy;

    for (uint tile_y = 0; tile_y < REDUCE_TILE_DIM; tile_y += REDUCE_BOUNDS_BLOCK_Y) {
        for (uint tile_x = 0; tile_x < REDUCE_TILE_DIM; tile_x += REDUCE_BOUNDS_BLOCK_X) {
            let coords = tile_start + uint2(tile_x, tile_y);

            if (coords.x < depth_dim.x && coords.y < depth_dim.y) {
                let depth_value = depth_texture.Load(int2(coords.x, coords.y), 0);
                let linear_view_z = depth::nonLinearToLinear(depth_value);

                // Check if sample falls within view frustum.
                if (linear_view_z > near_z && linear_view_z < far_z) {
                    // Determine which partition this sample belongs to.
                    var partition_index = 0;
                    [unroll]
                    for (uint i = 0; i < (PARTITION_COUNT - 1); i++) {
                        if (linear_view_z >= partition_data[i].interval_end) {
                            partition_index++;
                        }
                    }

                    // Reconstruct world position and transform to light space.
                    let world_pos = reconstruct_position_from_depth(coords, depth_value, depth_dim, global_uniforms);
                    let light_pos = transform_to_light_space(world_pos, directional_light.view_projection);

                    // Update bounds for this partition.
                    bounds_reduce[partition_index].min_coord = min(bounds_reduce[partition_index].min_coord, light_pos);
                    bounds_reduce[partition_index].max_coord = max(bounds_reduce[partition_index].max_coord, light_pos);
                }
            }
        }
    }

    [unroll]
    for (uint partition_index = 0; partition_index < PARTITION_COUNT; partition_index++) {
        let index = input.local_invocation_index * PARTITION_COUNT + partition_index;
        sbounds_min[index] = bounds_reduce[partition_index].min_coord;
        sbounds_max[index] = bounds_reduce[partition_index].max_coord;
    }

    GroupMemoryBarrierWithGroupSync();

    // Parallel reduction in shared memory.
    for (uint offset = REDUCE_BOUNDS_SHARED_MEMORY_ARRAY_SIZE >> 1; offset >= PARTITION_COUNT; offset >>= 1) {
        for (uint i = input.local_invocation_index; i < offset; i += REDUCE_BOUNDS_BLOCK_SIZE) {
            sbounds_min[i] = min(sbounds_min[i], sbounds_min[offset + i]);
            sbounds_max[i] = max(sbounds_max[i], sbounds_max[offset + i]);
        }
        GroupMemoryBarrierWithGroupSync();
    }

    if (input.local_invocation_index < PARTITION_COUNT) {
        // Apply the offset to move values into the positive range [0, 2]. We need to do this, since
        // using uint's min/max only works when the float are positive.
        let offset_min_vec = sbounds_min[input.local_invocation_index] + ATOMIC_FLOAT_OFFSET;
        let offset_max_vec = sbounds_max[input.local_invocation_index] + ATOMIC_FLOAT_OFFSET;

        bounds_data[input.local_invocation_index].min_coord_x.min(asuint(offset_min_vec.x), MemoryOrder::Relaxed);
        bounds_data[input.local_invocation_index].min_coord_y.min(asuint(offset_min_vec.y), MemoryOrder::Relaxed);
        bounds_data[input.local_invocation_index].min_coord_z.min(asuint(offset_min_vec.z), MemoryOrder::Relaxed);

        bounds_data[input.local_invocation_index].max_coord_x.max(asuint(offset_max_vec.x), MemoryOrder::Relaxed);
        bounds_data[input.local_invocation_index].max_coord_y.max(asuint(offset_max_vec.y), MemoryOrder::Relaxed);
        bounds_data[input.local_invocation_index].max_coord_z.max(asuint(offset_max_vec.z), MemoryOrder::Relaxed);
    }
}
