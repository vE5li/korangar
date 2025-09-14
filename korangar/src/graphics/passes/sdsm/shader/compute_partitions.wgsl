struct Partition {
    extents: vec4<f32>,
    center: vec4<f32>,
    interval_begin: f32,
    interval_end: f32,
}

struct Interval {
    begin: atomic<u32>,
    end: atomic<u32>,
}

const PARTITION_COUNT: u32 = 3u;
const NEAR_PLANE: f32 = 0.1;
const VIRTUAL_FAR_PLANE: f32 = 1000.0;

@group(1) @binding(2) var<storage, read_write> partition_data: array<Partition>;
@group(1) @binding(3) var<storage, read_write> interval_data: array<Interval>;

// PSSM partitioning scheme: Blend between logarithmic and uniform distribution.
// GPU Gems 3: Parallel-Split Shadow Maps on Programmable GPUs (2008).
fn pssm_partition_from_range(partition_index: u32, min_z: f32, max_z: f32) -> f32 {
    let BLEND_FACTOR: f32 = 0.5;
    let ratio = max_z / min_z;
    let power = f32(partition_index) / f32(PARTITION_COUNT);
    let log_split = min_z * pow(ratio, power);
    let uniform_split = min_z + (max_z - min_z) * (f32(partition_index) / f32(PARTITION_COUNT));
    return mix(uniform_split, log_split, BLEND_FACTOR);
}

@compute @workgroup_size(4, 1, 1)
fn cs_main(
    @builtin(global_invocation_id) globalId: vec3<u32>,
) {
    if (globalId.x < PARTITION_COUNT && globalId.y == 0u && globalId.z == 0u) {
        let idx = globalId.x;
        let min_z = bitcast<f32>(atomicLoad(&interval_data[0].begin));
        let max_z = bitcast<f32>(atomicLoad(&interval_data[PARTITION_COUNT - 1u].end));

        // We ensure that it still covers the whole range of the framebuffer (expand first and last).
        // This does not affect the solution at all since we derive the bounds based on the samples,
        // not the partition frusta.

        if (idx == 0u) {
            partition_data[idx].interval_begin = NEAR_PLANE;
        } else {
            partition_data[idx].interval_begin = pssm_partition_from_range(idx, min_z, max_z);
        }

        if (idx == PARTITION_COUNT - 1u) {
            partition_data[idx].interval_end = VIRTUAL_FAR_PLANE;
        } else {
            partition_data[idx].interval_end = pssm_partition_from_range(idx + 1u, min_z, max_z);
        }
    }
}
