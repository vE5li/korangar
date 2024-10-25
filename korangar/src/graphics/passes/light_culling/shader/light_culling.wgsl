struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
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

fn screen_to_clip_space(screen_space_position: vec2<f32>) -> vec2<f32> {
    let x = screen_space_position.x * 2.0 - 1.0;
    let y = -(screen_space_position.y * 2.0 - 1.0);
    return vec2<f32>(x, y);
}

fn calculate_tile_vectors(tile_x: u32, tile_y: u32) -> mat4x3<f32> {
    let screen_size = vec2<f32>(global_uniforms.screen_size);

    // Calculate tile corners in screen space
    let tile_min = vec2<f32>(
        f32(tile_x * TILE_SIZE) / screen_size.x,
        f32(tile_y * TILE_SIZE) / screen_size.y
    );
    let tile_max = vec2<f32>(
        f32((tile_x + 1u) * TILE_SIZE) / screen_size.x,
        f32((tile_y + 1u) * TILE_SIZE) / screen_size.y
    );

    // Convert to clip space
    let ndc_min = screen_to_clip_space(tile_min);
    let ndc_max = screen_to_clip_space(tile_max);

    // Create the four corner points
    let corners = mat4x4<f32>(
        vec4<f32>(ndc_min.x, ndc_min.y, 1.0, 1.0),
        vec4<f32>(ndc_max.x, ndc_min.y, 1.0, 1.0),
        vec4<f32>(ndc_max.x, ndc_max.y, 1.0, 1.0),
        vec4<f32>(ndc_min.x, ndc_max.y, 1.0, 1.0)
    );

    // Transform to world space and create side vectors
    var sides: mat4x3<f32>;

    var view_pos = global_uniforms.inverse_view_projection * corners[0];
    sides[0] = normalize(view_pos.xyz / view_pos.w);
    view_pos = global_uniforms.inverse_view_projection * corners[1];
    sides[1] = normalize(view_pos.xyz / view_pos.w);
    view_pos = global_uniforms.inverse_view_projection * corners[2];
    sides[2] = normalize(view_pos.xyz / view_pos.w);
    view_pos = global_uniforms.inverse_view_projection * corners[3];
    sides[3] = normalize(view_pos.xyz / view_pos.w);

    return sides;
}

/// Tests if a light sphere intersects with a view frustum tile's cone (Uses the "expanded cone" approach).
fn test_light_intersection(
    light_position: vec3<f32>,
    light_range: f32,
    tile_center_axis: vec3<f32>,
    tile_angle: f32
) -> bool {
    // Calculate distance to light
    let light_distance_sqr = dot(light_position, light_position);
    let light_distance = sqrt(light_distance_sqr);
    let light_center_unit_vector = light_position / light_distance;

    // Calculate expanded cone angle
    let expansion_angle = asin(min(light_range / light_distance, 1.0));
    let expanded_angle = tile_angle + expansion_angle;
    let expanded_angle_cos = cos(expanded_angle);

    // Test if light center is inside expanded cone using dot product
    return dot(light_center_unit_vector, tile_center_axis) >= expanded_angle_cos;
}

@compute @workgroup_size(8, 8, 1)
fn cs_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let num_tiles_x = (global_uniforms.screen_size.x + TILE_SIZE - 1u) / TILE_SIZE;
    let num_tiles_y = (global_uniforms.screen_size.y + TILE_SIZE - 1u) / TILE_SIZE;

    if (global_id.x >= num_tiles_x || global_id.y >= num_tiles_y) {
        return;
    }

    if (global_uniforms.point_light_count == 0u) {
        textureStore(light_count_texture, vec2<i32>(global_id.xy), vec4<u32>(0u, 0u, 0u, 0u));
        return;
    }

    let tile_index = global_id.y * num_tiles_x + global_id.x;

    // Calculate tile cone axis and angle
    let sides = calculate_tile_vectors(global_id.x, global_id.y);
    let cone_center_axis = normalize(sides[0] + sides[1] + sides[2] + sides[3]);
    let tile_cos = min(
        min(dot(normalize(sides[0]), cone_center_axis), dot(normalize(sides[1]), cone_center_axis)),
        min(dot(normalize(sides[2]), cone_center_axis), dot(normalize(sides[3]), cone_center_axis))
    );
    let cone_angle = acos(tile_cos);

    var local_count = 0u;

    // Test each light against the tile cone
    for (var i = 0u; i < global_uniforms.point_light_count; i++) {
        let light = lights[i];

        let is_intersecting = test_light_intersection(light.position.xyz, light.range, cone_center_axis, cone_angle);

        if (local_count < MAX_LIGHTS_PER_TILE) && is_intersecting {
            tile_light_indices[tile_index].indices[local_count] = i;
            local_count += 1u;
        }
    }

    textureStore(light_count_texture, vec2<i32>(global_id.xy), vec4<u32>(local_count, 0u, 0u, 0u));
}
