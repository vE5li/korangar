#language slang 2026

import sdsm;

struct ComputeInput {
    uint3 global_invocation_id : SV_DispatchThreadID;
}

[[vk::binding(2, 1)]] var partition_data: RWStructuredBuffer<Partition>;
[[vk::binding(4, 1)]] var bounds_data: RWStructuredBuffer<Bounds>;

[[shader("compute")]]
[[numthreads(4, 1, 1)]]
func main(ComputeInput input) {
    if (input.global_invocation_id.x < PARTITION_COUNT && input.global_invocation_id.y == 0 && input.global_invocation_id.z == 0) {
        let partition_index = input.global_invocation_id.x;
        let bounds = read_bounds_as_float(partition_index, bounds_data);

        let extents = (bounds.max_coord - bounds.min_coord) * 0.5;
        let center = (bounds.max_coord + bounds.min_coord) * 0.5;

        // Extents represents the half size of the bounding box in light space (NDC units).
        // e.g. a unit cube would have an extents of float3(0.5, 0.5, 0.5).
        partition_data[partition_index].extents = float4(extents, 1.0);

        // Center remains the same - we expand uniformly around the center.
        partition_data[partition_index].center = float4(center, 0.0);
    }
}
