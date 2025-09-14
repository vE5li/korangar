struct Interval {
    begin: atomic<u32>,
    end: atomic<u32>,
}

const PARTITION_COUNT: u32 = 3u;

@group(1) @binding(3) var<storage, read_write> interval_data: array<Interval>;

@compute @workgroup_size(4, 1, 1)
fn cs_main(
    @builtin(global_invocation_id) globalId: vec3<u32>,
) {
    if (globalId.x < PARTITION_COUNT && globalId.y == 0u && globalId.z == 0u) {
        atomicStore(&interval_data[globalId.x].begin, 0x7F7FFFFFu); // f32::MAX as u32
        atomicStore(&interval_data[globalId.x].end, 0u);
    }
}
