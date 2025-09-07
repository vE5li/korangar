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

struct InstanceData {
    color: vec4<f32>,
    corner_diameter: vec4<f32>,
    screen_clip: vec4<f32>,
    shadow_color: vec4<f32>,
    shadow_padding: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    rectangle_type: u32,
    texture_index: i32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) instance_index: u32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(1) @binding(1) var msdf_font_map: texture_2d<f32>;
@group(2) @binding(0) var texture: texture_2d<f32>;

// Because of how sampling works, we need to add a bit breathing room
// for the SDF function and then need to compensate for it.
const BREATHING_ROOM = 0.5;
const BORDER_THRESHOLD = 2.0;
const EDGE_VALUE: f32 = 0.5;
/// We use a pxrange of 6 px when creating MSDFs.
const PXRANGE: f32 = 6.0;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let instance = instance_data[instance_index];
    var vertex = vertex_data(vertex_index);

    let pixel_size = vec2<f32>(1.0 / f32(global_uniforms.interface_size.x), 1.0 / f32(global_uniforms.interface_size.y));
    let size_adjustment = select(vec2<f32>(0.0), (BREATHING_ROOM * 2.0) * pixel_size, any(instance.corner_diameter != vec4<f32>(0.0)));
    let shadow_padding = vec4<f32>(
        instance.shadow_padding.x * pixel_size.x,  // left padding
        instance.shadow_padding.y * pixel_size.x,  // right padding
        instance.shadow_padding.z * pixel_size.y,  // top padding
        instance.shadow_padding.w * pixel_size.y   // bottom padding
    );

    var adjusted_size = instance.screen_size + size_adjustment;

    // Only shift the quad for positive padding values (shadow extending outward).
    let position_offset = vec2<f32>(
        -max(0.0, shadow_padding.x),
        -max(0.0, shadow_padding.z)
    );

    // Expand the vertex bounds based on shadow padding.
    // For positive left/top padding: we shifted, so need to compensate on right/bottom.
    // For negative left/top padding: no shift, so just use the absolute values.
    let shadow_expansion = vec2<f32>(
        select(0.0, max(0.0, shadow_padding.x), vertex.x < 0.5) +
        select(0.0, shadow_padding.y + max(0.0, shadow_padding.x), vertex.x > 0.5),
        select(0.0, max(0.0, shadow_padding.z), vertex.y > -0.5) +
        select(0.0, shadow_padding.w + max(0.0, shadow_padding.z), vertex.y < -0.5)
    );
    adjusted_size += shadow_expansion;

    let clip_size = adjusted_size * 2.0;
    let position = screen_to_clip_space(instance.screen_position + position_offset) + vertex.xy * clip_size;

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.fragment_position = clip_to_screen_space(position);
    output.texture_coordinates = instance.texture_position + vertex.zw * instance.texture_size;
    output.instance_index = instance_index;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instance_data[input.instance_index];

    let clip_adjustment = select(vec4<f32>(0.0), vec4<f32>(-0.5, -0.5, 0.5, 0.5), any(instance.corner_diameter != vec4<f32>(0.0)));
    let adjusted_clip = instance.screen_clip + clip_adjustment;

    if (input.position.x < adjusted_clip.x || input.position.y < adjusted_clip.y ||
        input.position.x > adjusted_clip.z || input.position.y > adjusted_clip.w) {
        return vec4<f32>(0.0);
    }

    var color: vec4<f32> = instance.color;

    switch (instance.rectangle_type) {
        case 1u: {
            // Sprite (linear filtering)
            color *= textureSample(texture, linear_sampler, input.texture_coordinates);
        }
        case 2u: {
            // Sprite (nearest filtering)
            color *= textureSample(texture, nearest_sampler, input.texture_coordinates);
        }
        case 3u: {
            // SDF
            let distance = textureSample(texture, linear_sampler, input.texture_coordinates).r;
            color *= vec4(saturate((distance - 0.5) * 2.0 / fwidth(distance)));
        }
        case 4u: {
            // Text
            let distances = textureSample(msdf_font_map, linear_sampler, input.texture_coordinates);
            let distance = median(distances.r, distances.g, distances.b) - EDGE_VALUE;
            let texture_size = vec2<f32>(textureDimensions(msdf_font_map));
            color = calculate_msdf(distance, texture_size, input.texture_coordinates, color);
        }
        default: {}
    }

    if (instance.rectangle_type == 0u && any(instance.shadow_padding != vec4<f32>(0.0))) {
        // Solid rectangle with shadow.
        return render_rectangle_with_shadow(
            instance.corner_diameter,
            instance.screen_position,
            instance.screen_size,
            input.fragment_position,
            color,
            instance.shadow_color,
            instance.shadow_padding,
        );
    } else {
        // All other shadowless rectangles.
        return rectangle_with_rounded_edges(
            instance.corner_diameter,
            instance.screen_position,
            instance.screen_size,
            input.fragment_position,
            color,
        );
    }
}

fn calculate_msdf(
    distance: f32,
    texture_size: vec2<f32>,
    texture_coordinates: vec2<f32>,
    color: vec4<f32>
) -> vec4<f32> {
    let unit_range = (vec2<f32>(PXRANGE) / texture_size);
    let screen_texture_size = vec2<f32>(1.0) / fwidth(texture_coordinates);
    let screen_px_range = max(EDGE_VALUE * dot(unit_range, screen_texture_size), 1.0);

    return color * saturate(distance * screen_px_range + EDGE_VALUE);
}

fn calculate_rounded_rectangle_alpha(
    pixel_position: vec2<f32>,
    rectangle_origin: vec2<f32>,
    rectangle_size: vec2<f32>,
    corner_diamerter: vec4<f32>,
    shadow_spread: f32,
) -> f32 {
    // Calculate position relative to rectangle center.
    let half_size = rectangle_size * 0.5;
    let rectangle_center = rectangle_origin + half_size;
    let relative_position = pixel_position - rectangle_center;

    // Determine which corner diameter to use based on the quadrant this fragment is in.
    let is_right = relative_position.x > 0.0;
    let is_bottom = relative_position.y > 0.0;
    let diamerter_pair = select(corner_diamerter.xy, corner_diamerter.zw, is_bottom);
    let corner_diameter = select(diamerter_pair.x, diamerter_pair.y, is_right);

    if (corner_diameter == 0.0 && shadow_spread == 0.0) {
        // No rounded corners - simple bounds check.
        return f32(abs(relative_position.x) <= half_size.x &&
                   abs(relative_position.y) <= half_size.y);
    }

    // Calculate SDF distance for rounded corners.
    let distance = rectangle_sdf(relative_position, half_size, corner_diameter);

    if (shadow_spread > 0.0) {
        return 1.0 - smoothstep(-4.0, 0.0, distance / (shadow_spread / 4.0));

    } else {
        // Multi-sample for anti-aliasing at edges.
        var alpha = step(0.0, -distance);

        if (abs(distance) <= BORDER_THRESHOLD) {
            var total = alpha;
            for (var index = 0u; index < 8u; index++) {
                let offset = SAMPLE_OFFSETS[index];
                let sample_distance = rectangle_sdf(relative_position + offset, half_size, corner_diameter);
                total += step(0.0, -sample_distance);
            }
            alpha = total * (1.0 / 9.0);
        }
        return alpha;
    }
}

fn render_rectangle_with_shadow(
    corner_diamerter: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    fragment_position: vec2<f32>,
    color: vec4<f32>,
    raw_shadow_color: vec4<f32>,
    shadow_padding: vec4<f32>,
) -> vec4<f32> {
    let interface_size = vec2<f32>(global_uniforms.interface_size);
    let pixel_position = fragment_position * interface_size;

    let main_origin = (screen_position * interface_size) - vec2<f32>(BREATHING_ROOM);
    let main_size = (screen_size * interface_size) + vec2<f32>(BREATHING_ROOM * 2.0);

    let main_alpha = calculate_rounded_rectangle_alpha(
        pixel_position,
        main_origin,
        main_size,
        corner_diamerter,
        0.0
    );

    if (main_alpha >= 1.0) {
        // If fully inside main rectangle.
        return color;
    }

    let shadow_origin = vec2<f32>(
        main_origin.x - shadow_padding.x,
        main_origin.y - shadow_padding.z
    );
    let shadow_size = vec2<f32>(
        main_size.x + shadow_padding.x + shadow_padding.y,
        main_size.y + shadow_padding.z + shadow_padding.w
    );

    let corner_adjustment = vec4<f32>(
        (shadow_padding.x + shadow_padding.z) / 2.0,
        (shadow_padding.y + shadow_padding.z) / 2.0,
        (shadow_padding.x + shadow_padding.w) / 2.0,
        (shadow_padding.y + shadow_padding.w) / 2.0,
    );

    let shadow_raw_alpha = calculate_rounded_rectangle_alpha(
        pixel_position,
        shadow_origin,
        shadow_size,
        corner_diamerter + corner_adjustment,
        max(max(max(shadow_padding.x, shadow_padding.y), shadow_padding.z), shadow_padding.w),
    );

    let shadow_color = raw_shadow_color * shadow_raw_alpha;

    if (main_alpha > 0.0) {
        // Main rectangle partially or fully covers this pixel.
        let main_contrib = color * main_alpha;
        let shadow_contrib = shadow_color * (1.0 - main_alpha);
        return main_contrib + shadow_contrib;
    } else {
        // Only shadow visible at this pixel.
        return shadow_color;
    }
}

fn rectangle_with_rounded_edges(
    corner_diamerter: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    fragment_position: vec2<f32>,
    color: vec4<f32>,
) -> vec4<f32> {
    if (all(corner_diamerter == vec4<f32>(0.0))) {
        return color;
    }

    // Convert normalized screen space coordinates to pixel space.
    let interface_size = vec2<f32>(global_uniforms.interface_size);
    let pixel_position = fragment_position * interface_size;
    let origin = (screen_position * interface_size) - vec2<f32>(BREATHING_ROOM);
    let size = (screen_size * interface_size) + vec2<f32>(BREATHING_ROOM * 2.0);

    let alpha = calculate_rounded_rectangle_alpha(
        pixel_position,
        origin,
        size,
        corner_diamerter,
        0.0
    );

    return color * alpha;
}

// 8-point Poisson Disk pattern that showed the best performance / quality characteristic.
const SAMPLE_OFFSETS: array<vec2<f32>, 8> = array<vec2<f32>, 8>(
    vec2<f32>( 0.924,  0.382) * 0.5,
    vec2<f32>( 0.382,  0.924) * 0.5,
    vec2<f32>(-0.382,  0.924) * 0.5,
    vec2<f32>(-0.924,  0.382) * 0.5,
    vec2<f32>(-0.924, -0.382) * 0.5,
    vec2<f32>(-0.382, -0.924) * 0.5,
    vec2<f32>( 0.382, -0.924) * 0.5,
    vec2<f32>( 0.924, -0.382) * 0.5
);

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  w
// 0             0  0  0  0
// 1             1  0  1  0
// 2             1 -1  1  1
// 3             1 -1  1  1
// 4             0 -1  0  1
// 5             0  0  0  0
//
// (x,y) are the vertex position
// (z,w) are the UV coordinates
fn vertex_data(vertex_index: u32) -> vec4<f32> {
    let index = 1u << vertex_index;
    let x = f32((index & 0xEu) != 0u);
    let y = f32((index & 0x1Cu) != 0u);
    return vec4<f32>(x, -y, x, y);
}

fn screen_to_clip_space(screen_coords: vec2<f32>) -> vec2<f32> {
    let x = (screen_coords.x * 2.0) - 1.0;
    let y = -(screen_coords.y * 2.0) + 1.0;
    return vec2<f32>(x, y);
}

fn clip_to_screen_space(ndc: vec2<f32>) -> vec2<f32> {
    let u = (ndc.x + 1.0) / 2.0;
    let v = (1.0 - ndc.y) / 2.0;
    return vec2<f32>(u, v);
}

// Calculation based on:
// "Leveraging Rust and the GPU to render user interfaces at 120 FPS"
// https://zed.dev/blog/videogame
fn rectangle_sdf(
    relative_position: vec2<f32>,
    half_size: vec2<f32>,
    corner_diameter: f32
) -> f32 {
    let shrunk_corner_position = half_size - corner_diameter;
    let pixel_to_shrunk_corner = max(vec2<f32>(0.0), abs(relative_position) - shrunk_corner_position);
    return length(pixel_to_shrunk_corner) - corner_diameter + BREATHING_ROOM;
}

fn median(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}
