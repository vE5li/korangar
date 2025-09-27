#language slang 2026

import sdsm;

struct ComputeInput {
    uint3 global_invocation_id : SV_DispatchThreadID;
}

[[vk::binding(4, 1)]] var bounds_data: RWStructuredBuffer<Bounds>;

[[shader("compute")]]
[[numthreads(4, 1, 1)]]
func main(ComputeInput input) {
    if (input.global_invocation_id.x < PARTITION_COUNT && input.global_invocation_id.y == 0 && input.global_invocation_id.z == 0) {
        bounds_data[input.global_invocation_id.x].min_coord_x.store(0x7F7FFFFF, MemoryOrder::Relaxed); // f32::MAX as u32
        bounds_data[input.global_invocation_id.x].min_coord_y.store(0x7F7FFFFF, MemoryOrder::Relaxed);
        bounds_data[input.global_invocation_id.x].min_coord_z.store(0x7F7FFFFF, MemoryOrder::Relaxed);
        bounds_data[input.global_invocation_id.x].max_coord_x.store(0, MemoryOrder::Relaxed);
        bounds_data[input.global_invocation_id.x].max_coord_y.store(0, MemoryOrder::Relaxed);
        bounds_data[input.global_invocation_id.x].max_coord_z.store(0, MemoryOrder::Relaxed);
    }
}
