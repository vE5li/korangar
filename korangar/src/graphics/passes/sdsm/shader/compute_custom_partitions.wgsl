struct Partition {
    extents: vec4<f32>,
    center: vec4<f32>,
    interval_begin: f32,
    interval_end: f32,
}

struct Bounds {
    min_coord_x: atomic<u32>,
    min_coord_y: atomic<u32>,
    min_coord_z: atomic<u32>,
    max_coord_x: atomic<u32>,
    max_coord_y: atomic<u32>,
    max_coord_z: atomic<u32>,
}

struct BoundsFloat {
    min_coord: vec3<f32>,
    max_coord: vec3<f32>,
}

const PARTITION_COUNT: u32 = 3u;
const ATOMIC_FLOAT_OFFSET: f32 = 1.0;

@group(1) @binding(2) var<storage, read_write> partition_data: array<Partition>;
@group(1) @binding(4) var<storage, read_write> bounds_data: array<Bounds>;

fn read_bounds_as_float(partition_index: u32) -> BoundsFloat {
    let min_coord_offset = vec3<f32>(
        bitcast<f32>(atomicLoad(&bounds_data[partition_index].min_coord_x)),
        bitcast<f32>(atomicLoad(&bounds_data[partition_index].min_coord_y)),
        bitcast<f32>(atomicLoad(&bounds_data[partition_index].min_coord_z))
    );

    let max_coord_offset = vec3<f32>(
        bitcast<f32>(atomicLoad(&bounds_data[partition_index].max_coord_x)),
        bitcast<f32>(atomicLoad(&bounds_data[partition_index].max_coord_y)),
        bitcast<f32>(atomicLoad(&bounds_data[partition_index].max_coord_z))
    );

    // Subtract the offset to return to the original range.
    let min_coord = min_coord_offset - ATOMIC_FLOAT_OFFSET;
    let max_coord = max_coord_offset - ATOMIC_FLOAT_OFFSET;

    return BoundsFloat(min_coord, max_coord);
}

@compute @workgroup_size(4, 1, 1)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    if (global_id.x < PARTITION_COUNT && global_id.y == 0u && global_id.z == 0u) {
        let partition_index = global_id.x;
        let bounds = read_bounds_as_float(partition_index);

        let extents = (bounds.max_coord - bounds.min_coord) * 0.5;
        let center = (bounds.max_coord + bounds.min_coord) * 0.5;

        // Extends represents the half size of the bounding box in light space (NDC units).
        // e.g. a unit cube would have an extend of vec3(0.5, 0.5, 0.5).
        partition_data[partition_index].extents = vec4<f32>(extents, 1.0);

        // Center remains the same - we expand uniformly around the center
        partition_data[partition_index].center = vec4<f32>(center, 0.0);
    }
}
