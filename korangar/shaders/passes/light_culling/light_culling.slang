#language slang 2026

import globals;
import transform;
import matrix;

public struct PointLight {
    public var position: float4;
    public var color: float4;
    public var range: float;
    public var texture_index: int;
}

public struct TileLightIndices {
    public var indices: uint[256];
}

static const var TILE_SIZE: uint = 16;
static const var MAX_LIGHTS_PER_TILE: uint = 256;

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(0, 1)]] var lights: StructuredBuffer<PointLight>;
[[vk::binding(1, 1)]] [[vk::image_format("r32ui")]] var light_count_texture: WTexture2D<uint>;
[[vk::binding(2, 1)]] var tile_light_indices: RWStructuredBuffer<TileLightIndices>;

[[shader("compute")]]
[[numthreads(8, 8, 1)]]
func main(uint3 global_id : SV_DispatchThreadID) {
    let tile_count_x = (global_uniforms.forward_size.x + TILE_SIZE - 1) / TILE_SIZE;
    let tile_count_y = (global_uniforms.forward_size.y + TILE_SIZE - 1) / TILE_SIZE;

    if (global_id.x >= tile_count_x || global_id.y >= tile_count_y) {
        return;
    }

    if (global_uniforms.point_light_count == 0) {
        light_count_texture.Store(global_id.xy, 0);
        return;
    }

    let tile_index = global_id.y * tile_count_x + global_id.x;

    // Calculate cone axis in view space.
    let sides = calculate_tile_vectors(global_id.x, global_id.y);
    let view_space_cone_axis = normalize(sides.getColumn(0).xyz + sides.getColumn(1).xyz + sides.getColumn(2).xyz + sides.getColumn(3).xyz);

    // Transform cone axis and origin to world space.
    let world_space_cone_axis = mul(global_uniforms.inverse_view, float4(view_space_cone_axis.x, view_space_cone_axis.y, view_space_cone_axis.z, 0.0)).xyz;
    let world_space_cone_origin = mul(global_uniforms.inverse_view, float4(0.0, 0.0, 0.0, 1.0)).xyz;

    // Calculate the cone's angle.
    let cone_angle_cos = min(
        min(dot(view_space_cone_axis, sides.getColumn(0).xyz), dot(view_space_cone_axis, sides.getColumn(1).xyz)),
        min(dot(view_space_cone_axis, sides.getColumn(2).xyz), dot(view_space_cone_axis, sides.getColumn(3).xyz)),
    );
    let cone_angle_tan = sqrt(1.0 / (cone_angle_cos * cone_angle_cos) - 1.0);

    let cone_rotation_matrix = create_cone_rotation_matrix(world_space_cone_axis);

    var local_count = 0;

    // Test each light against the tile cone.
    for (var index = 0; index < global_uniforms.point_light_count; index++) {
        let light = lights[index];

        // Position the light's center relative to the new origin
        // and also rotate it, so that it axis aligned.
        let light_relative_position = light.position.xyz - world_space_cone_origin;
        let light_aligned_position = mul(cone_rotation_matrix, light_relative_position);

        let is_intersecting = intersect_cone_sphere_aligned(
            light_aligned_position,
            light.range,
            cone_angle_tan,
        );

        if ((local_count < MAX_LIGHTS_PER_TILE) && is_intersecting) {
            tile_light_indices[tile_index].indices[local_count] = index;
            local_count += 1;
        }
    }

    light_count_texture.Store(global_id.xy, local_count);
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
[ForceInline]
func intersect_cone_sphere_aligned(
    sphere_center: float3,
    sphere_radius: float,
    cone_angle_tan: float
) -> bool {
    let cone_radius = max(cone_angle_tan * sphere_center.z, 0.0);
    let extends_past_cone_tip = sphere_center.z > -sphere_radius;
    let intersects_cone = length(sphere_center.xy) <= cone_radius + sphere_radius;
    return extends_past_cone_tip && intersects_cone;
}

[ForceInline]
func calculate_tile_vectors(tile_x: uint, tile_y: uint) -> float4x4 {
    let forward_size = float2(global_uniforms.forward_size);

    // Calculate tile corners in screen space (0 to 1).
    let tile_min = float2(
        float(tile_x * TILE_SIZE) / forward_size.x,
        float(tile_y * TILE_SIZE) / forward_size.y
    );
    let tile_max = float2(
        float((tile_x + 1) * TILE_SIZE) / forward_size.x,
        float((tile_y + 1) * TILE_SIZE) / forward_size.y
    );

    // Convert to NDC space (-1 to 1).
    let ndc_min = coordinate_space::screen_to_clip_space(tile_min);
    let ndc_max = coordinate_space::screen_to_clip_space(tile_max);

    // Create the four corner points in NDC.
    // Row based constructor!
    let corners = float4x4(
        float4(ndc_min.x, ndc_min.y, 1.0, 1.0),
        float4(ndc_max.x, ndc_min.y, 1.0, 1.0),
        float4(ndc_max.x, ndc_max.y, 1.0, 1.0),
        float4(ndc_min.x, ndc_max.y, 1.0, 1.0)
    );

    // Transform corners to view space and create the side vectors.
    // sic! Somehow dxc doesn't like it if we do a matrix multiplication.
    let view_corner_0 = mul(global_uniforms.inverse_projection, corners.getRow(0));
    let view_corner_1 = mul(global_uniforms.inverse_projection, corners.getRow(1));
    let view_corner_2 = mul(global_uniforms.inverse_projection, corners.getRow(2));
    let view_corner_3 = mul(global_uniforms.inverse_projection, corners.getRow(3));

    let side0 = normalize(view_corner_0.xyz / view_corner_0.w);
    let side1 = normalize(view_corner_1.xyz / view_corner_1.w);
    let side2 = normalize(view_corner_2.xyz / view_corner_2.w);
    let side3 = normalize(view_corner_3.xyz / view_corner_3.w);

    // Row based constructor!
    let sides = float4x4(
        side0.x, side1.x, side2.x, side3.x,
        side0.y, side1.y, side2.y, side3.y,
        side0.z, side1.z, side2.z, side3.z,
        0, 0, 0, 0
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
[ForceInline]
func create_cone_rotation_matrix(cone_axis: float3) -> float3x3 {
    let forward_axis = normalize(cone_axis);
    let reference_vector = abs(forward_axis.x) > 0.9 ? float3(0.0, 1.0, 0.0) : float3(1.0, 0.0, 0.0);
    let right_axis = normalize(cross(reference_vector, forward_axis));
    let up_axis = cross(forward_axis, right_axis);
    // Row based constructor!
    return float3x3(right_axis, up_axis, forward_axis);
}
