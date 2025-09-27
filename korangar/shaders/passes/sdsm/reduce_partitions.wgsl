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
const REDUCE_ZBOUNDS_BLOCK_DIM: u32 = 16u;
const REDUCE_ZBOUNDS_BLOCK_SIZE: u32 = REDUCE_ZBOUNDS_BLOCK_DIM * REDUCE_ZBOUNDS_BLOCK_DIM;
const NEAR_PLANE: f32 = 0.1;
const VIRTUAL_FAR_PLANE: f32 = 1000.0;
const DEPTH_EPSILON: f32 = 1.0e-7;
const REDUCE_TILE_DIM: u32 = 64u;

@group(1) @binding(1) var depth_texture: texture_depth_2d;
@group(1) @binding(2) var<storage, read_write> partition_data: array<Partition>;
@group(1) @binding(3) var<storage, read_write> interval_data: array<Interval>;

var<workgroup> smin_z: array<f32, REDUCE_ZBOUNDS_BLOCK_SIZE>;
var<workgroup> smax_z: array<f32, REDUCE_ZBOUNDS_BLOCK_SIZE>;

fn nonLinearToLinear(non_linear_depth: f32) -> f32 {
    return NEAR_PLANE / (non_linear_depth + DEPTH_EPSILON);
}

@compute @workgroup_size(REDUCE_ZBOUNDS_BLOCK_DIM, REDUCE_ZBOUNDS_BLOCK_DIM, 1)
fn main(
    @builtin(global_invocation_id) globalId: vec3<u32>,
    @builtin(local_invocation_id) localId: vec3<u32>,
    @builtin(workgroup_id) groupId: vec3<u32>,
    @builtin(local_invocation_index) localIndex: u32,
) {
    let depth_dim = textureDimensions(depth_texture);

    var min_z: f32 = VIRTUAL_FAR_PLANE;
    var max_z: f32 = NEAR_PLANE;

    let tile_start = groupId.xy * REDUCE_TILE_DIM + localId.xy;

    for (var tile_y: u32 = 0u; tile_y < REDUCE_TILE_DIM; tile_y += REDUCE_ZBOUNDS_BLOCK_DIM) {
        for (var tile_x: u32 = 0u; tile_x < REDUCE_TILE_DIM; tile_x += REDUCE_ZBOUNDS_BLOCK_DIM) {
            let coords = tile_start + vec2<u32>(tile_x, tile_y);

            if (coords.x < depth_dim.x && coords.y < depth_dim.y) {
                let depth = textureLoad(depth_texture, coords, 0);
                let linear_view_z = nonLinearToLinear(depth);

                if (linear_view_z >= NEAR_PLANE && linear_view_z < VIRTUAL_FAR_PLANE) {
                    min_z = min(min_z, linear_view_z);
                    max_z = max(max_z, linear_view_z);
                }
            }
        }
    }

    smin_z[localIndex] = min_z;
    smax_z[localIndex] = max_z;

    workgroupBarrier();

    // Parallel reduction in shared memory.
    for (var offset: u32 = REDUCE_ZBOUNDS_BLOCK_SIZE >> 1u; offset > 0u; offset >>= 1u) {
        if (localIndex < offset) {
            smin_z[localIndex] = min(smin_z[localIndex], smin_z[offset + localIndex]);
            smax_z[localIndex] = max(smax_z[localIndex], smax_z[offset + localIndex]);
        }
        workgroupBarrier();
    }

    // Write out the result from this workgroup using atomics.
    if (localIndex == 0u) {
        atomicMin(&interval_data[0].begin, bitcast<u32>(smin_z[0]));
        atomicMax(&interval_data[PARTITION_COUNT - 1u].end, bitcast<u32>(smax_z[0]));
    }
}
