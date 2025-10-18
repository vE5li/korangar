#language slang 2026

import depth_texture;
import sdsm;
import transform;

struct ComputeInput {
    uint3 global_invocation_id : SV_DispatchThreadID;
    uint3 local_invocation_id : SV_GroupThreadID;
    uint3 workgroup_id : SV_GroupID;
    uint local_invocation_index : SV_GroupIndex;
}

[[vk::binding(1, 1)]] var depth_texture: DepthTexture2DMS;
[[vk::binding(2, 1)]] var partition_data: RWStructuredBuffer<Partition>;
[[vk::binding(3, 1)]] var interval_data: RWStructuredBuffer<Interval>;

groupshared float smin_z[REDUCE_ZBOUNDS_BLOCK_SIZE];
groupshared float smax_z[REDUCE_ZBOUNDS_BLOCK_SIZE];

[[shader("compute")]]
[[numthreads(REDUCE_ZBOUNDS_BLOCK_DIM, REDUCE_ZBOUNDS_BLOCK_DIM, 1)]]
func main(ComputeInput input) {
    var depth_dim: uint2;
    var sample_count: uint;
    depth_texture.GetDimensions(depth_dim.x, depth_dim.y, sample_count);

    var min_z = VIRTUAL_FAR_PLANE;
    var max_z = NEAR_PLANE;

    let tile_start = input.workgroup_id.xy * REDUCE_TILE_DIM + input.local_invocation_id.xy;

    [unroll]
    for (uint tile_y = 0; tile_y < REDUCE_TILE_DIM; tile_y += REDUCE_ZBOUNDS_BLOCK_DIM) {
        [unroll]
        for (uint tile_x = 0; tile_x < REDUCE_TILE_DIM; tile_x += REDUCE_ZBOUNDS_BLOCK_DIM) {
            let coords = tile_start + uint2(tile_x, tile_y);

            if (coords.x < depth_dim.x && coords.y < depth_dim.y) {
                let depth_value = depth_texture.Load(int2(coords.x, coords.y), 0);
                let linear_view_z = depth::nonLinearToLinear(depth_value);

                if (linear_view_z >= NEAR_PLANE && linear_view_z < VIRTUAL_FAR_PLANE) {
                    min_z = min(min_z, linear_view_z);
                    max_z = max(max_z, linear_view_z);
                }
            }
        }
    }

    smin_z[input.local_invocation_index] = min_z;
    smax_z[input.local_invocation_index] = max_z;

    GroupMemoryBarrierWithGroupSync();

    // Parallel reduction in shared memory.
    [unroll]
    for (uint offset = REDUCE_ZBOUNDS_BLOCK_SIZE >> 1; offset > 0; offset >>= 1) {
        if (input.local_invocation_index < offset) {
            smin_z[input.local_invocation_index] = min(smin_z[input.local_invocation_index], smin_z[offset + input.local_invocation_index]);
            smax_z[input.local_invocation_index] = max(smax_z[input.local_invocation_index], smax_z[offset + input.local_invocation_index]);
        }
        GroupMemoryBarrierWithGroupSync();
    }

    // Write out the result from this workgroup using atomics.
    if (input.local_invocation_index == 0) {
        interval_data[0].begin.min(asuint(smin_z[0]), MemoryOrder::Relaxed);
        interval_data[PARTITION_COUNT - 1].end.max(asuint(smax_z[0]), MemoryOrder::Relaxed);
    }
}
