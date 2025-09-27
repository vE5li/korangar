struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    camera_position: vec4<f32>,
    forward_size: vec2<u32>,
    interface_size: vec2<u32>,
    pointer_position: vec2<u32>,
    animation_timer: f32,
    day_timer: f32,
    point_light_count: u32,
    enhanced_lighting: u32,
    shadow_quality: u32,
}

struct DirectionalLightUniforms {
    view_projection: mat4x4<f32>,
    color: vec4<f32>,
    direction: vec4<f32>,
}

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
const REDUCE_BOUNDS_BLOCK_X: u32 = 16u;
const REDUCE_BOUNDS_BLOCK_Y: u32 = 8u;
const REDUCE_BOUNDS_BLOCK_SIZE: u32 = REDUCE_BOUNDS_BLOCK_X * REDUCE_BOUNDS_BLOCK_Y;
const REDUCE_BOUNDS_SHARED_MEMORY_ARRAY_SIZE: u32 = PARTITION_COUNT * REDUCE_BOUNDS_BLOCK_SIZE;
const REDUCE_TILE_DIM: u32 = 64u;
const NEAR_PLANE: f32 = 0.1;
const DEPTH_EPSILON: f32 = 1.0e-7;
const ATOMIC_FLOAT_OFFSET: f32 = 1.0;

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(1) @binding(0) var<uniform> directional_light: DirectionalLightUniforms;
@group(1) @binding(1) var depth_texture: texture_depth_multisampled_2d;
@group(1) @binding(2) var<storage, read_write> partition_data: array<Partition>;
@group(1) @binding(4) var<storage, read_write> bounds_data: array<Bounds>;

var<workgroup> sbounds_min: array<vec3<f32>, REDUCE_BOUNDS_SHARED_MEMORY_ARRAY_SIZE>;
var<workgroup> sbounds_max: array<vec3<f32>, REDUCE_BOUNDS_SHARED_MEMORY_ARRAY_SIZE>;

fn nonLinearToLinear(non_linear_depth: f32) -> f32 {
    return NEAR_PLANE / (non_linear_depth + DEPTH_EPSILON);
}

fn reconstruct_position_from_depth(coords: vec2<u32>, depth: f32, depth_dim: vec2<u32>) -> vec3<f32> {
    // Convert screen coordinates to NDC.
    let uv = vec2<f32>(coords) / vec2<f32>(depth_dim);
    let ndc_xy = uv * 2.0 - 1.0;
    let ndc = vec4<f32>(ndc_xy.x, -ndc_xy.y, depth, 1.0);

    // Transform to view space.
    let view_pos = global_uniforms.inverse_projection * ndc;
    let view_pos_3d = view_pos.xyz / view_pos.w;

    // Transform to world space.
    let world_pos = global_uniforms.inverse_view * vec4<f32>(view_pos_3d, 1.0);

    return world_pos.xyz;
}

fn transform_to_light_space(world_pos: vec3<f32>) -> vec3<f32> {
    let light_clip = directional_light.view_projection * vec4<f32>(world_pos, 1.0);
    return light_clip.xyz;
}

fn empty_bounds_float() -> BoundsFloat {
    return BoundsFloat(
        vec3<f32>(3.40282347E+38, 3.40282347E+38, 3.40282347E+38),
        vec3<f32>(0.0, 0.0, 0.0)
    );
}

@compute @workgroup_size(REDUCE_BOUNDS_BLOCK_X, REDUCE_BOUNDS_BLOCK_Y, 1)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) group_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32,
) {
    let depth_dim = textureDimensions(depth_texture);

    var bounds_reduce: array<BoundsFloat, PARTITION_COUNT>;
    for (var partition_index: u32 = 0u; partition_index < PARTITION_COUNT; partition_index++) {
        bounds_reduce[partition_index] = empty_bounds_float();
    }

    let near_z = partition_data[0].interval_begin;
    let far_z = partition_data[PARTITION_COUNT - 1u].interval_end;

    let tile_start = group_id.xy * REDUCE_TILE_DIM + local_id.xy;

    for (var tile_y: u32 = 0u; tile_y < REDUCE_TILE_DIM; tile_y += REDUCE_BOUNDS_BLOCK_Y) {
        for (var tile_x: u32 = 0u; tile_x < REDUCE_TILE_DIM; tile_x += REDUCE_BOUNDS_BLOCK_X) {
            let coords = tile_start + vec2<u32>(tile_x, tile_y);

            if (coords.x < depth_dim.x && coords.y < depth_dim.y) {
                let depth = textureLoad(depth_texture, coords, 0);
                let linear_view_z = nonLinearToLinear(depth);

                // Check if sample falls within view frustum.
                if (linear_view_z > near_z && linear_view_z < far_z) {
                    // Determine which partition this sample belongs to.
                    var partition_index: u32 = 0u;
                    for (var i: u32 = 0u; i < (PARTITION_COUNT - 1u); i++) {
                        if (linear_view_z >= partition_data[i].interval_end) {
                            partition_index++;
                        }
                    }

                    // Reconstruct world position and transform to light space.
                    let world_pos = reconstruct_position_from_depth(coords, depth, depth_dim);
                    let light_pos = transform_to_light_space(world_pos);

                    // Update bounds for this partition.
                    bounds_reduce[partition_index].min_coord = min(bounds_reduce[partition_index].min_coord, light_pos);
                    bounds_reduce[partition_index].max_coord = max(bounds_reduce[partition_index].max_coord, light_pos);
                }
            }
        }
    }

    for (var partition_index: u32 = 0u; partition_index < PARTITION_COUNT; partition_index++) {
        let index = local_index * PARTITION_COUNT + partition_index;
        sbounds_min[index] = bounds_reduce[partition_index].min_coord;
        sbounds_max[index] = bounds_reduce[partition_index].max_coord;
    }

    workgroupBarrier();

    // Parallel reduction in shared memory.
    for (var offset: u32 = REDUCE_BOUNDS_SHARED_MEMORY_ARRAY_SIZE >> 1u; offset >= PARTITION_COUNT; offset >>= 1u) {
        for (var i: u32 = local_index; i < offset; i += REDUCE_BOUNDS_BLOCK_SIZE) {
            sbounds_min[i] = min(sbounds_min[i], sbounds_min[offset + i]);
            sbounds_max[i] = max(sbounds_max[i], sbounds_max[offset + i]);
        }
        workgroupBarrier();
    }

    if (local_index < PARTITION_COUNT) {
        // Apply the offset to move values into the positive range [0, 2]. We need to do this, since
        // using u32's min/max only works when the f32 are positive.
        let offset_min_vec = sbounds_min[local_index] + ATOMIC_FLOAT_OFFSET;
        let offset_max_vec = sbounds_max[local_index] + ATOMIC_FLOAT_OFFSET;

        atomicMin(&bounds_data[local_index].min_coord_x, bitcast<u32>(offset_min_vec.x));
        atomicMin(&bounds_data[local_index].min_coord_y, bitcast<u32>(offset_min_vec.y));
        atomicMin(&bounds_data[local_index].min_coord_z, bitcast<u32>(offset_min_vec.z));

        atomicMax(&bounds_data[local_index].max_coord_x, bitcast<u32>(offset_max_vec.x));
        atomicMax(&bounds_data[local_index].max_coord_y, bitcast<u32>(offset_max_vec.y));
        atomicMax(&bounds_data[local_index].max_coord_z, bitcast<u32>(offset_max_vec.z));
    }
}
