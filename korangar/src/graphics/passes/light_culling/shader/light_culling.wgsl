struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    screen_size: vec2<u32>,
    pointer_position: vec2<u32>,
    animation_timer: f32,
    day_timer: f32,
    water_level: f32,
    point_light_count: u32,
}

struct PointLight {
    position: vec4<f32>,
    color: vec4<f32>,
    range: f32,
    texture_index: i32,
}

struct TileLightIndices {
    indices: array<u32, 256>,
}

const TILE_SIZE: u32 = 16;
const MAX_LIGHTS_PER_TILE: u32 = 256;

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(1) @binding(0) var<storage, read> lights: array<PointLight>;
@group(1) @binding(1) var light_count_texture: texture_storage_2d<r32uint, write>;
@group(1) @binding(2) var<storage, read_write> tile_light_indices: array<TileLightIndices>;

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tile_count_x = (global_uniforms.screen_size.x + TILE_SIZE - 1u) / TILE_SIZE;
    let tile_count_y = (global_uniforms.screen_size.y + TILE_SIZE - 1u) / TILE_SIZE;

    if (global_id.x >= tile_count_x || global_id.y >= tile_count_y) {
        return;
    }

    if (global_uniforms.point_light_count == 0u) {
        textureStore(light_count_texture, vec2<i32>(global_id.xy), vec4<u32>(0u, 0u, 0u, 0u));
        return;
    }

    let tile_index = global_id.y * tile_count_x + global_id.x;

    // Calculate cone axis in view space
    let sides = calculate_tile_vectors(global_id.x, global_id.y);
    let view_space_cone_axis = normalize(sides[0] + sides[1] + sides[2] + sides[3]);

    // Transform cone axis and origin to world space
    let world_space_cone_axis = (global_uniforms.inverse_view * vec4<f32>(view_space_cone_axis, 0.0)).xyz;
    let world_space_cone_origin = (global_uniforms.inverse_view * vec4<f32>(0.0, 0.0, 0.0, 1.0)).xyz;

    // Calculate the cone's angle
    let cone_angle_cos = min(
        min(dot(view_space_cone_axis, sides[0]), dot(view_space_cone_axis, sides[1])),
        min(dot(view_space_cone_axis, sides[2]), dot(view_space_cone_axis, sides[3])),
    );
    let cone_angle_tan = sqrt(1.0 / (cone_angle_cos * cone_angle_cos) - 1.0);

    let cone_rotation_matrix = create_cone_rotation_matrix(world_space_cone_axis);

    var local_count = 0u;

    // Test each light against the tile cone
    for (var index = 0u; index < global_uniforms.point_light_count; index++) {
        let light = lights[index];

        // Position the light's center relative to the new origin
        // and also rotate it, so that it axis aligned.
        let light_relativ_position = light.position.xyz - world_space_cone_origin;
        let light_aligned_position = cone_rotation_matrix * light_relativ_position;

        let is_intersecting = intersect_cone_sphere_aligned(
            light_aligned_position,
            light.range,
            cone_angle_tan,
        );

        if (local_count < MAX_LIGHTS_PER_TILE) && is_intersecting {
            tile_light_indices[tile_index].indices[local_count] = index;
            local_count += 1u;
        }
    }

    textureStore(light_count_texture, vec2<i32>(global_id.xy), vec4<u32>(local_count, 0u, 0u, 0u));
}

/// Tests if a sphere intersects with a cone that is aligned to the +Z axis with its tip at the origin.
/// The spheres we test must be inside the cone's view space. This is an optimized version that takes
/// advantage of the special case where the cone is Z-aligned.
///
/// # How it works
///
/// The test is split into two parts:
///
/// 1. `extends_past_cone_tip`: Checks if any part of the sphere extends past z=0 (the cone's tip)
///    - A sphere extends past z=0 if its closest point to the XY plane is at or behind z=0
///    - This occurs when: sphere_center.z > -sphere_radius
///
/// 2. `intersects_cone`: Tests if the sphere intersects the cone's surface at the sphere's Z position
///    - At any Z position, the cone forms a circular cross-section
///    - The radius of this cross-section is: max(cone_angle_tan * sphere_center.z, 0.0)
///    - length(sphere_center.xy) gives the distance from sphere center to cone's axis
///    - For intersection, this distance must be <= cone_radius + sphere_radius
///
/// Based on an original idea from Jonathan W. Hale's bachelor thesis
/// "Dual-Cone View Culling for Virtual Reality Applications" (2018).
fn intersect_cone_sphere_aligned(
    sphere_center: vec3<f32>,
    sphere_radius: f32,
    cone_angle_tan: f32
) -> bool {
    let cone_radius = max(cone_angle_tan * sphere_center.z, 0.0);
    let extends_past_cone_tip = sphere_center.z > -sphere_radius;
    let intersects_cone = length(sphere_center.xy) <= cone_radius + sphere_radius;
    return extends_past_cone_tip && intersects_cone;
}

fn calculate_tile_vectors(tile_x: u32, tile_y: u32) -> mat4x3<f32> {
    let screen_size = vec2<f32>(global_uniforms.screen_size);

    // Calculate tile corners in screen space (0 to 1)
    let tile_min = vec2<f32>(
        f32(tile_x * TILE_SIZE) / screen_size.x,
        f32(tile_y * TILE_SIZE) / screen_size.y
    );
    let tile_max = vec2<f32>(
        f32((tile_x + 1u) * TILE_SIZE) / screen_size.x,
        f32((tile_y + 1u) * TILE_SIZE) / screen_size.y
    );

    // Convert to NDC space (-1 to 1)
    let ndc_min = screen_to_clip_space(tile_min);
    let ndc_max = screen_to_clip_space(tile_max);

    // Create the four corner points in NDC
    let corners = mat4x4<f32>(
        vec4<f32>(ndc_min.x, ndc_min.y, 1.0, 1.0),
        vec4<f32>(ndc_max.x, ndc_min.y, 1.0, 1.0),
        vec4<f32>(ndc_max.x, ndc_max.y, 1.0, 1.0),
        vec4<f32>(ndc_min.x, ndc_max.y, 1.0, 1.0)
    );

    // Transform corners to view space and create the side vectors.
    // sic! Somehow dxc doesn't like it if we do a matrix multiplication.
    let view_corner_0 = global_uniforms.inverse_projection * corners[0];
    let view_corner_1 = global_uniforms.inverse_projection * corners[1];
    let view_corner_2 = global_uniforms.inverse_projection * corners[2];
    let view_corner_3 = global_uniforms.inverse_projection * corners[3];

    let sides = mat4x3<f32>(
        normalize(view_corner_0.xyz / view_corner_0.w),
        normalize(view_corner_1.xyz / view_corner_1.w),
        normalize(view_corner_2.xyz / view_corner_2.w),
        normalize(view_corner_3.xyz / view_corner_3.w)
    );

    return sides;
}

/// Creates a rotation matrix by constructing an orthonormal basis where the +Z axis
/// aligns with the given cone_axis. This uses a Gram-Schmidt-like process to build
/// a coordinate frame by:
/// 1. Using the cone_axis as the forward (Z) direction
/// 2. Choosing a reference vector perpendicular to the cone_axis
/// 3. Computing right (X) and up (Y) vectors via cross products
///
/// The resulting 3x3 matrix transforms from the standard basis to this new basis,
/// effectively rotating any vector into the cone's coordinate system.
fn create_cone_rotation_matrix(cone_axis: vec3<f32>) -> mat3x3<f32> {
    let forward_axis = normalize(cone_axis);
    let reference_vector = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), abs(forward_axis.x) > 0.9);
    let right_axis = normalize(cross(reference_vector, forward_axis));
    let up_axis = cross(forward_axis, right_axis);
    return mat3x3<f32>(
        right_axis.x, up_axis.x, forward_axis.x,
        right_axis.y, up_axis.y, forward_axis.y,
        right_axis.z, up_axis.z, forward_axis.z
    );
}

fn screen_to_clip_space(screen_space_position: vec2<f32>) -> vec2<f32> {
    let x = screen_space_position.x * 2.0 - 1.0;
    let y = -(screen_space_position.y * 2.0 - 1.0);
    return vec2<f32>(x, y);
}
