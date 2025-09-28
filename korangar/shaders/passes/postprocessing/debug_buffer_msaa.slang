#language slang 2026

import depth_texture;
import globals;
import screen_space;
import transform;

public struct DebugUniforms {
    public var show_picker_buffer: uint;
    public var show_directional_shadow_map: uint;
    public var show_point_shadow_map: uint;
    public var show_light_culling_count_buffer: uint;
    public var show_sdsm_partitions: uint;
    public var show_font_map: uint;
}

public struct Partition {
    public var scale: float4;
    public var bias: float4;
    public var interval_begin: float;
    public var interval_end: float;
}

static const var TILE_SIZE: uint = 16;

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(1, 0)]] var nearest_sampler: SamplerState;
[[vk::binding(0, 1)]] var debug_uniforms: ConstantBuffer<DebugUniforms>;
[[vk::binding(1, 1)]] var picker_texture: Texture2D<uint2>;
[[vk::binding(2, 1)]] var directional_shadow_maps: DepthTexture2DArray;
[[vk::binding(3, 1)]] var light_count_texture: Texture2D<uint>;
[[vk::binding(4, 1)]] var point_shadow_maps: DepthTextureCubeArray;
[[vk::binding(5, 1)]] var depth_texture: DepthTexture2DMS;
[[vk::binding(6, 1)]] var partition_data: StructuredBuffer<Partition>;
[[vk::binding(0, 2)]] var font_map: Texture2D;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex.new(vertex_index);
}

[[shader("pixel")]]
func fs_main(input: FullscreenVertex) -> float4 {
    let pixel_coord = int2(input.position.xy);

    var output_color = float4(0.0);

    if (debug_uniforms.show_picker_buffer != 0) {
        let picker = picker_texture.Load(int3(pixel_coord, 0)).rg;
        let red = float(picker.r & 0xf) / 100.0;
        let green = float((picker.r >> 8) & 0xf) / 100.0;
        let blue = float((picker.r >> 16) & 0xf) / 100.0;
        output_color += float4(red, green, blue, 1.0);
    }

    if (debug_uniforms.show_directional_shadow_map != 0) {
        let sample_position = coordinate_space::clip_to_screen_space(input.uv);
        let depth = directional_shadow_maps.Sample(nearest_sampler, float3(sample_position, float(debug_uniforms.show_directional_shadow_map - 1)));
        output_color += float4(depth, depth, depth, 1.0);
    }

    if (debug_uniforms.show_point_shadow_map != 0) {
        // +--------+--------+--------+
        // |   +Y   |   +Z   |   +X   |
        // +--------+--------+--------+
        // |   -X   |   -Z   |   -Y   |
        // +--------+--------+--------+

        // Calculate the grid cell (0-5) based on the fragment position.
        let grid_x = floor(input.uv.x * 1.5 + 1.5);
        let grid_y = floor(input.uv.y + 1.0);
        let grid_index = int(grid_x + grid_y * 3.0);

        // Calculate UV coordinates within the cell.
        let cell_uv = float2(
            frac(input.uv.x * 1.5 + 1.5),
            frac(input.uv.y + 1.0)
        );

        // Map the 2D cell UV to a direction vector for the cube face.
        var sample_dir = float3(0.0);
        switch (grid_index) {
            case 0:
                sample_dir = float3(1.0, -cell_uv.y * 2.0 + 1.0, -cell_uv.x * 2.0 + 1.0);
                break;
            case 1:
                sample_dir = float3(-1.0, -cell_uv.y * 2.0 + 1.0, cell_uv.x * 2.0 - 1.0);
                break;
            case 2:
                sample_dir = float3(cell_uv.x * 2.0 - 1.0, 1.0, cell_uv.y * 2.0 - 1.0);
                break;
            case 3:
                sample_dir = float3(cell_uv.x * 2.0 - 1.0, -1.0, -cell_uv.y * 2.0 + 1.0);
                break;
            case 4:
                sample_dir = float3(cell_uv.x * 2.0 - 1.0, -cell_uv.y * 2.0 + 1.0, 1.0);
                break;
            default:
                sample_dir = float3(-cell_uv.x * 2.0 + 1.0, -cell_uv.y * 2.0 + 1.0, -1.0);
                break;
        }

        let depth = point_shadow_maps.Sample(nearest_sampler, float4(sample_dir, float(debug_uniforms.show_point_shadow_map - 1))).r;
        output_color += float4(depth, depth, depth, 1.0);
    }

    if (debug_uniforms.show_light_culling_count_buffer != 0) {
        let tile_coord = pixel_coord / int(TILE_SIZE);
        let count = light_count_texture.Load(int3(tile_coord, 0)).r;

        var color = float4(0.0);

        if (count != 0) {
            if (count <= 7) {
                let incidence = float(count) / 7.0;
                color = float4(0.0, incidence, 1.0 - incidence, 1.0) * 0.25;
            } else if (count <= 13) {
                let incidence = (float(count) - 7.0) / 6.0;
                color = float4(incidence, 1.0, 0.0, 1.0) * 0.25;
            } else if (count <= 20) {
                let incidence = (float(count) - 13.0) / 7.0;
                color = float4(1.0, 1.0 - incidence, 0.0, 1.0) * 0.25;
            } else {
                color = float4(1.0, 0.0, 0.0, 1.0) * 0.25;
            }
        }

        output_color += color;
    }

    if (debug_uniforms.show_sdsm_partitions != 0) {
        let depth_value = depth_texture.Load(pixel_coord, 0);
        let linear_depth = depth::nonLinearToLinear(depth_value);
        var partition_color = float4(0.0);

        if (linear_depth >= partition_data[0].interval_begin && linear_depth < partition_data[0].interval_end) {
            partition_color = float4(1.0, 0.0, 0.0, 1.0);
        } else if (linear_depth >= partition_data[1].interval_begin && linear_depth < partition_data[1].interval_end) {
            partition_color = float4(0.0, 1.0, 0.0, 1.0);
        } else if (linear_depth >= partition_data[2].interval_begin && linear_depth < partition_data[2].interval_end) {
            partition_color = float4(0.0, 0.0, 1.0, 1.0);
        }

        output_color += partition_color * 0.25;
    }

    if (debug_uniforms.show_font_map != 0) {
        let color = font_map.Sample(nearest_sampler, coordinate_space::clip_to_screen_space(input.uv)).rgb;
        output_color += float4(color, 1.0);
    }

    return output_color;
}
