#language slang 2026

import sdsm;
import transform;

struct ComputeInput {
    uint3 global_invocation_id : SV_DispatchThreadID;
}

[[vk::binding(2, 1)]] var partition_data: RWStructuredBuffer<Partition>;
[[vk::binding(3, 1)]] var interval_data: RWStructuredBuffer<Interval>;

[[shader("compute")]]
[[numthreads(4, 1, 1)]]
func main(ComputeInput input) {
    if (input.global_invocation_id.x < PARTITION_COUNT && input.global_invocation_id.y == 0 && input.global_invocation_id.z == 0) {
        var idx = input.global_invocation_id.x;

        // Read the min/max z values from interval data.
        let min_z = asfloat(interval_data[0].begin.load(MemoryOrder::Relaxed));
        let max_z = asfloat(interval_data[PARTITION_COUNT - 1].end.load(MemoryOrder::Relaxed));

        // We ensure that it still covers the whole range of the framebuffer (expand first and last).
        // This does not affect the solution at all since we derive the bounds based on the samples,
        // not the partition frusta.

        if (idx == 0) {
            partition_data[idx].interval_begin = NEAR_PLANE;
        } else {
            partition_data[idx].interval_begin = pssm_partition_from_range(idx, min_z, max_z);
        }

        if (idx == PARTITION_COUNT - 1) {
            partition_data[idx].interval_end = VIRTUAL_FAR_PLANE;
        } else {
            partition_data[idx].interval_end = pssm_partition_from_range(idx + 1, min_z, max_z);
        }
    }
}
