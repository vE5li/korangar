#language slang 2026

import globals;
import matrix;
import directional_shadow;

struct IndicatorVertexInput {
	uint vertex_index : SV_VulkanVertexID;
};

struct IndicatorVertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] var texture_coordinates: float2;
};

struct IndicatorFragmentOutput {
    [[vk::location(0)]] var translucence: float;
};

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var pass_uniforms: ConstantBuffer<PassUniforms>;
[[vk::binding(0, 2)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(input: IndicatorVertexInput) -> IndicatorVertexOutput {
    var output: IndicatorVertexOutput;
    output.position = mul(pass_uniforms.view_projection, position_data(input.vertex_index));
    output.texture_coordinates = uv_data(input.vertex_index);
    return output;
}

[[shader("pixel")]]
func fs_main(input: IndicatorVertexOutput) -> IndicatorFragmentOutput {
    let fragment_color = texture.SampleLevel(linear_sampler, input.texture_coordinates, 0.0);

    if (fragment_color.a == 0.0) {
        discard;
    }

    var output: IndicatorFragmentOutput;
    output.translucence = 1.0;
    return output;
}

[ForceInline]
func position_data(vertex_index: uint) -> float4 {
    switch (vertex_index) {
        case 0: {
            // upper_left
            return global_uniforms.indicator_positions.getColumn(0);
        }
        case 1: {
            // upper_right
            return global_uniforms.indicator_positions.getColumn(1);
        }
        case 2: {
            // lower_left
            return global_uniforms.indicator_positions.getColumn(2);
        }
        case 3: {
            // lower_left
            return global_uniforms.indicator_positions.getColumn(2);
        }
        case 4: {
            // upper_right
            return global_uniforms.indicator_positions.getColumn(1);
        }
        default: {
            // lower_right
            return global_uniforms.indicator_positions.getColumn(3);
        }
    }
}

// Optimized version of the following truth table:
//
// vertex_index  x  y
// 0             0  0
// 1             0  1
// 2             1  0
// 3             1  0
// 4             0  1
// 5             1  1
//
// (x,y) are the UV coordinates
[ForceInline]
func uv_data(vertex_index: uint) -> float2 {
    let index = 1 << vertex_index;
    let x = float((index & 0x2C) != 0);
    let y = float((index & 0x32) != 0);
    return float2(x, y);
}
