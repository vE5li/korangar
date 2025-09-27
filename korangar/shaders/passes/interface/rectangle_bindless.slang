#language slang 2026

import globals;
import interface;

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(1, 0)]] var nearest_sampler: SamplerState;
[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<RectangleInstanceData>;
[[vk::binding(1, 1)]] var msdf_font_map: Texture2D;
[[vk::binding(2, 1)]] var textures: Texture2D[];

[[shader("vertex")]]
func vs_main(input: RectangleVertexInput) -> RectangleVertexOutput {
    return rectangle_vertex_shader(input.vertex_index, input.instance_index, global_uniforms, instance_data);
}

[[shader("pixel")]]
func fs_main(input: RectangleVertexOutput) -> float4 {
    let instance = instance_data[input.instance_index];

    let clip_adjustment = any(instance.corner_diameter != float4(0.0)) ? float4(-0.5, -0.5, 0.5, 0.5) : float4(0.0);
    let adjusted_clip = instance.screen_clip + clip_adjustment;

    if (input.position.x < adjusted_clip.x || input.position.y < adjusted_clip.y ||
        input.position.x > adjusted_clip.z || input.position.y > adjusted_clip.w
    ) {
        return float4(0.0, 0.0, 0.0, 0.0);
    }

    var color = instance.color;

    switch (instance.rectangle_type) {
        case 1: {
            // Sprite (linear filtering)
            color *= textures[instance.texture_index].Sample(linear_sampler, input.texture_coordinates);
            break;
        }
        case 2: {
            // Sprite (nearest filtering)
            color *= textures[instance.texture_index].Sample(nearest_sampler, input.texture_coordinates);
            break;
        }
        case 3: {
            // SDF
            float distance = textures[instance.texture_index].Sample(linear_sampler, input.texture_coordinates).r;
            color *= float4(saturate((distance - 0.5) * 2.0 / fwidth(distance)));
            break;
        }
        case 4: {
            // Text
            let distances = msdf_font_map.Sample(linear_sampler, input.texture_coordinates);
            let distance = median(distances.r, distances.g, distances.b) - EDGE_VALUE;
            var texture_size: float2;
            msdf_font_map.GetDimensions(texture_size.x, texture_size.y);
            color = calculate_msdf(distance, texture_size, input.texture_coordinates, color);
            break;
        }
        default: {
            break;
        }
    }

    let interface_size = float2(global_uniforms.interface_size);

    if (instance.rectangle_type == 0 && any(instance.shadow_padding != float4(0.0))) {
        // Solid rectangle with shadow.
        return render_rectangle_with_shadow(
            instance.corner_diameter,
            instance.screen_position,
            instance.screen_size,
            input.fragment_position,
            color,
            instance.shadow_color,
            instance.shadow_padding,
            interface_size);
    } else {
        // All other shadowless rectangles.
        return rectangle_with_rounded_edges(
            instance.corner_diameter,
            instance.screen_position,
            instance.screen_size,
            input.fragment_position,
            color,
            interface_size);
    }
}
