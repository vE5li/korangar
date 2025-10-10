#language slang 2026

module interface;

import globals;
import transform;

static const var BREATHING_ROOM: float = 0.5;
public static const var EDGE_VALUE: float = 0.5;
public static const var PXRANGE: float = 6.0;

public struct RectangleInstanceData {
    public var color: float4;
    public var corner_diameter: float4;
    public var screen_clip: float4;
    public var shadow_color: float4;
    public var shadow_padding: float4;
    public var screen_position: float2;
    public var screen_size: float2;
    public var texture_position: float2;
    public var texture_size: float2;
    public var rectangle_type: uint;
    public var texture_index: int;
};

public struct RectangleVertexInput {
	public uint vertex_index : SV_VulkanVertexID;
	public uint instance_index : SV_VulkanInstanceID;
};

public struct RectangleVertexOutput {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var fragment_position: float2;
    [[vk::location(1)]] public var texture_coordinates: float2;
    [[vk::location(2)]] public var instance_index: uint;
};

[ForceInline]
func vertex_data(vertex_index: uint) -> float4 {
    let index = 1 << vertex_index;
    let x = float((index & 0x0E) != 0);
    let y = float((index & 0x1C) != 0);
    return float4(x, -y, x, y);
}

[ForceInline]
func rectangle_sdf(relative_position: float2, half_size: float2, corner_diameter: float) -> float {
    let shrunk_corner_position = half_size - corner_diameter;
    let pixel_to_shrunk_corner = max(float2(0.0), abs(relative_position) - shrunk_corner_position);
    return length(pixel_to_shrunk_corner) - corner_diameter + BREATHING_ROOM;
}

// Analytical gradient of the rectangle SDF function.
//
// Derived through symbolic differentiation of the SDF formula:
//   sdf(p) = length(max(0, abs(p) - shrunk)) - corner_diameter
//
// The gradient was found using symbolic math (SymPy) and chain rule:
// 1. The derivative of abs(x) is sign(x) for x ≠ 0.
//    - At x = 0, abs(x) is non-differentiable (has a "corner").
//    - We use sign(x) as a subgradient, which is valid and works well in practice.
//    - This occurs exactly at the rectangle's axis-aligned edges.
// 2. The derivative of length(v) is v/length(v) (normalized vector).
// 3. The max(0, v) operation acts as a "gradient gate":
//    - When v <= 0 (inside rectangle): gradient is zero.
//    - When v > 0 (outside rectangle): gradient passes through.
//
// This gives us the final gradient:
//   ∇sdf = (d / |d|) * sign(p)
// where d = max(0, abs(p) - shrunk_corner_position)
[ForceInline]
func rectangle_sdf_gradient(relative_position: float2, half_size: float2, corner_diameter: float) -> float2 {
    let shrunk_corner_position = half_size - corner_diameter;
    let distance_to_shrunk_corner_unclipped = abs(relative_position) - shrunk_corner_position;
    let distance_to_shrunk_corner = max(float2(0.0), distance_to_shrunk_corner_unclipped);
    let distance_magnitude = length(distance_to_shrunk_corner) + 1e-7;
    return (distance_to_shrunk_corner / distance_magnitude) * sign(relative_position);
}

[ForceInline]
public func median(r: float, g: float, b: float) -> float {
    return max(min(r, g), min(max(r, g), b));
}

[ForceInline]
public func calculate_msdf(
    distance: float,
    texture_size: float2,
    texture_coordinates: float2,
    color: float4,
) -> float4 {
    let unit_range = (float2(PXRANGE) / texture_size);
    let screen_texture_size = float2(1.0) / fwidth(texture_coordinates);
    let screen_px_range = max(EDGE_VALUE * dot(unit_range, screen_texture_size), 1.0);
    return color * saturate(distance * screen_px_range + EDGE_VALUE);
}

[ForceInline]
public func calculate_rounded_rectangle_alpha(
    pixel_position: float2,
    rectangle_origin: float2,
    rectangle_size: float2,
    corner_diameter: float4,
    shadow_spread: float,
) -> float {
    // Calculate position relative to rectangle center.
    let half_size = rectangle_size * 0.5;
    let rectangle_center = rectangle_origin + half_size;
    let relative_position = pixel_position - rectangle_center;

    // Determine which corner diameter to use based on quadrant.
    let is_right = relative_position.x > 0.0;
    let is_bottom = relative_position.y > 0.0;
    let diameter_pair = is_bottom ? corner_diameter.zw : corner_diameter.xy;
    let corner_diam = is_right ? diameter_pair.y : diameter_pair.x;

    if (corner_diam == 0.0 && shadow_spread == 0.0) {
        // No rounded corners - simple bounds check.
        return float(abs(relative_position.x) <= half_size.x && abs(relative_position.y) <= half_size.y);
    }

    // Calculate SDF distance for rounded corners.
    let distance = rectangle_sdf(relative_position, half_size, corner_diam);

    if (shadow_spread > 0.0) {
        return 1.0 - smoothstep(-4.0, 0.0, distance / (shadow_spread / 4.0));
    } else {
        // Analytical anti-aliasing using gradient.
        // The gradient tells us how many pixels we move per unit of distance change.
        let gradient = rectangle_sdf_gradient(relative_position, half_size, corner_diam);

        // Calculate how fast the distance changes in screen space.
        let gradient_magnitude = length(gradient);

        // Convert distance to pixel-space units.
        let pixel_scale = 1.0 / max(gradient_magnitude, 0.001);
        let pixel_distance = distance * pixel_scale;

        // Smooth transition over 1 pixel.
        return saturate(0.5 - pixel_distance);
    }
}

[ForceInline]
public func render_rectangle_with_shadow(
    corner_diameter: float4,
    screen_position: float2,
    screen_size: float2,
    fragment_position: float2,
    color: float4,
    raw_shadow_color: float4,
    shadow_padding: float4,
    interface_size: float2,
) -> float4 {
    let pixel_position = fragment_position * interface_size;

    let main_origin = (screen_position * interface_size) - float2(BREATHING_ROOM);
    let main_size = (screen_size * interface_size) + float2(BREATHING_ROOM * 2.0);

    let main_alpha = calculate_rounded_rectangle_alpha(pixel_position, main_origin, main_size, corner_diameter, 0.0);

    if (main_alpha >= 1.0) {
        // Fully inside main rectangle.
        return color;
    }

    let shadow_origin = float2(
        main_origin.x - shadow_padding.x,
        main_origin.y - shadow_padding.z
    );
    let shadow_size = float2(
        main_size.x + shadow_padding.x + shadow_padding.y,
        main_size.y + shadow_padding.z + shadow_padding.w
    );
    let corner_adjustment = float4(
        (shadow_padding.x + shadow_padding.z) / 2.0,
        (shadow_padding.y + shadow_padding.z) / 2.0,
        (shadow_padding.x + shadow_padding.w) / 2.0,
        (shadow_padding.y + shadow_padding.w) / 2.0
    );

    let shadow_raw_alpha = calculate_rounded_rectangle_alpha(
        pixel_position,
        shadow_origin,
        shadow_size,
        corner_diameter + corner_adjustment,
        max(max(max(shadow_padding.x, shadow_padding.y), shadow_padding.z), shadow_padding.w)
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

[ForceInline]
public func rectangle_with_rounded_edges(
    corner_diameter: float4,
    screen_position: float2,
    screen_size: float2,
    fragment_position: float2,
    color: float4,
    interface_size: float2,
) -> float4 {
    if (all(corner_diameter == float4(0.0))) {
        return color;
    }

    // Convert normalized screen space coordinates to pixel space.
    let pixel_position = fragment_position * interface_size;
    let origin = (screen_position * interface_size) - float2(BREATHING_ROOM);
    let size = (screen_size * interface_size) + float2(BREATHING_ROOM * 2.0);

    let alpha = calculate_rounded_rectangle_alpha(pixel_position, origin, size, corner_diameter, 0.0);

    return color * alpha;
}

[ForceInline]
public func rectangle_vertex_shader(
    vertex_index: uint,
    instance_index: uint,
    global_uniforms: GlobalUniforms,
    instance_data: StructuredBuffer<RectangleInstanceData>,
) -> RectangleVertexOutput {
    let instance = instance_data[instance_index];
    let vertex = vertex_data(vertex_index);

    let pixel_size = float2(
        1.0 / float(global_uniforms.interface_size.x),
        1.0 / float(global_uniforms.interface_size.y)
    );
    let size_adjustment = any(instance.corner_diameter != float4(0.0)) ? (BREATHING_ROOM * 2.0) * pixel_size : float2(0.0);
    let shadow_padding = float4(
        instance.shadow_padding.x * pixel_size.x, // left padding
        instance.shadow_padding.y * pixel_size.x, // right padding
        instance.shadow_padding.z * pixel_size.y, // top padding
        instance.shadow_padding.w * pixel_size.y  // bottom padding
    );

    var adjusted_size = instance.screen_size + size_adjustment;

    // Only shift the quad for positive padding values (shadow extending outward).
    let position_offset = float2(-max(0.0, shadow_padding.x), -max(0.0, shadow_padding.z));

    // Expand the vertex bounds based on shadow padding.
    let shadow_expansion = float2(
        (vertex.x <  0.5 ? max(0.0, shadow_padding.x) : 0.0) + (vertex.x >  0.5 ? shadow_padding.y + max(0.0, shadow_padding.x) : 0.0),
        (vertex.y > -0.5 ? max(0.0, shadow_padding.z) : 0.0) + (vertex.y < -0.5 ? shadow_padding.w + max(0.0, shadow_padding.z) : 0.0)
    );
    adjusted_size += shadow_expansion;

    let clip_size = adjusted_size * 2.0;
    let position = coordinate_space::screen_to_clip_space(instance.screen_position + position_offset) + vertex.xy * clip_size;

    var output: RectangleVertexOutput;
    output.position = float4(position, 0.0, 1.0);
    output.fragment_position = coordinate_space::clip_to_screen_space(position);
    output.texture_coordinates = instance.texture_position + vertex.zw * instance.texture_size;
    output.instance_index = instance_index;
    return output;
}
