#language slang 2026

import sdsm;

struct ComputeInput {
    uint3 global_invocation_id : SV_DispatchThreadID;
}

[[vk::binding(3, 1)]] var interval_data: RWStructuredBuffer<Interval>;

[[shader("compute")]]
[[numthreads(4, 1, 1)]]
func main(ComputeInput input) {
    if (input.global_invocation_id.x < PARTITION_COUNT && input.global_invocation_id.y == 0 && input.global_invocation_id.z == 0) {
        interval_data[input.global_invocation_id.x].begin.store(0x7F7FFFFF, MemoryOrder::Relaxed); // f32::MAX as u32
        interval_data[input.global_invocation_id.x].end.store(0, MemoryOrder::Relaxed);
    }
}
