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

    // Calculate tile cone axis and angle
    let sides = calculate_tile_vectors(global_id.x, global_id.y);
    let cone_center_axis = normalize(sides[0] + sides[1] + sides[2] + sides[3]);
    let cone_angle_cos = min(
        min(dot(cone_center_axis, sides[0]), dot(cone_center_axis, sides[1])),
        min(dot(cone_center_axis, sides[2],), dot(cone_center_axis, sides[3]))
    );
    let cone_angle_tan = sqrt(1.0 / (cone_angle_cos * cone_angle_cos) - 1.0);

    var local_count = 0u;

    // Test each light against the tile cone
    for (var index = 0u; index < global_uniforms.point_light_count; index++) {
        let light = lights[index];

        let light_position = global_uniforms.view * vec4<f32>(light.position.xyz, 1.0);

        let is_intersecting = intersect_cone_sphere(
            cone_center_axis.xyz,
            cone_angle_tan,
            light_position.xyz,
            light.range,
        );

        if (local_count < MAX_LIGHTS_PER_TILE) && is_intersecting {
            tile_light_indices[tile_index].indices[local_count] = index;
            local_count += 1u;
        }
    }

    textureStore(light_count_texture, vec2<i32>(global_id.xy), vec4<u32>(local_count, 0u, 0u, 0u));
}

/// This is an intersection test that uses the Minkowski difference between the cone and sphere.
/// Here we are expanding the cone by sphere_radius to create a "fattened" cone that represents
/// all points where a sphere of that radius could intersect.
///
/// cone_center_axis and sphere_center must be in view space and the cone origin must be the
/// coordinate system origin. cone_center_axis must also be normalized.
///
/// Based on: "Intersection of a Sphere and a Cone" (2020) by David Eberly.
fn intersect_cone_sphere(
    cone_center_axis: vec3<f32>,
    cone_angle_tan: f32,
    sphere_center: vec3<f32>,
    sphere_radius: f32
) -> bool {
    // Check if sphere center is within radius of cone origin.
    let distance_to_origin_squared = dot(sphere_center, sphere_center);
    let in_origin_radius = distance_to_origin_squared <= sphere_radius * sphere_radius;

    let projected_length = dot(sphere_center, cone_center_axis);
    let distance_from_axis = length(sphere_center - projected_length * cone_center_axis);
    let expanded_radius = (projected_length * cone_angle_tan) + sphere_radius;

    // Check if point is inside the "fattened" cone and the projection is in front of the cone origin.
    let in_cone_region = distance_from_axis <= expanded_radius && projected_length >= -sphere_radius;

    return in_origin_radius || in_cone_region;
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

    // Transform corners to view space and create the side vectors
    // sic! Somehow dxc doesn't like it if we to a matrix multiplication
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

fn screen_to_clip_space(screen_space_position: vec2<f32>) -> vec2<f32> {
    let x = screen_space_position.x * 2.0 - 1.0;
    let y = -(screen_space_position.y * 2.0 - 1.0);
    return vec2<f32>(x, y);
}
