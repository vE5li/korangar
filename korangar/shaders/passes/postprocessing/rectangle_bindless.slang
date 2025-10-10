#language slang 2026

import postprocessing;

[[vk::binding(1, 0)]] var nearest_sampler: SamplerState;
[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<RectangleInstanceData>;
[[vk::binding(1, 1)]] var msdf_font_map: Texture2D;
[[vk::binding(2, 1)]] var textures: Texture2D[];

[[shader("vertex")]]
func vs_main(input: PostprocessingVertexInput) -> PostprocessingVertexOutput {
    let instance = instance_data[input.instance_index];

    let vertex = rectangle_vertex_data(input.vertex_index);

    let clip_size = instance.screen_size * 2.0;
    let position = screen_to_clip_space(instance.screen_position) + vertex.xy * clip_size;

    var output: PostprocessingVertexOutput;
    output.position = float4(position, 0.0, 1.0);
    output.texture_coordinates = instance.texture_position + vertex.zw * instance.texture_size;
    output.instance_index = input.instance_index;
    return output;
}

[[shader("pixel")]]
func fs_main(input: PostprocessingVertexOutput) -> float4 {
    let instance = instance_data[input.instance_index];

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
            let distance = textures[instance.texture_index].Sample(linear_sampler, input.texture_coordinates).r - EDGE_VALUE;
            var texture_size: float2;
            textures[instance.texture_index].GetDimensions(texture_size.x, texture_size.y);
            color = calculate_shadowed_sdf(distance, texture_size, input.texture_coordinates, color);
            break;
        }
        case 4: {
            // Text (MSDF)
            let distances = msdf_font_map.Sample(linear_sampler, input.texture_coordinates);
            let distance = median(distances.r, distances.g, distances.b) - EDGE_VALUE;
            var texture_size: float2;
            msdf_font_map.GetDimensions(texture_size.x, texture_size.y);
            color = calculate_shadowed_sdf(distance, texture_size, input.texture_coordinates, color);
            break;
        }
        default: {
            break;
        }
    }

    return color;
}
