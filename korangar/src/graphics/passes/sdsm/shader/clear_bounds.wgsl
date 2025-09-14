struct Bounds {
    min_coord_x: atomic<u32>,
    min_coord_y: atomic<u32>,
    min_coord_z: atomic<u32>,
    max_coord_x: atomic<u32>,
    max_coord_y: atomic<u32>,
    max_coord_z: atomic<u32>,
}

const PARTITION_COUNT: u32 = 3u;

@group(1) @binding(4) var<storage, read_write> bounds_data: array<Bounds>;

@compute @workgroup_size(4, 1, 1)
fn cs_main(
    @builtin(global_invocation_id) globalId: vec3<u32>,
) {
    if (globalId.x < PARTITION_COUNT && globalId.y == 0u && globalId.z == 0u) {
        atomicStore(&bounds_data[globalId.x].min_coord_x, 0x7F7FFFFFu);
        atomicStore(&bounds_data[globalId.x].min_coord_y, 0x7F7FFFFFu);
        atomicStore(&bounds_data[globalId.x].min_coord_z, 0x7F7FFFFFu);
        atomicStore(&bounds_data[globalId.x].max_coord_x, 0x0u);
        atomicStore(&bounds_data[globalId.x].max_coord_y, 0x0u);
        atomicStore(&bounds_data[globalId.x].max_coord_z, 0x0u);
    }
}
