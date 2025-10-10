#language slang 2026

import postprocessing;

public struct EffectVertexOutput {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var texture_coordinates: float2;
    [[vk::location(1)]] public var color: float4;
}

[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<EffectInstanceData>;
[[vk::binding(0, 2)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(input: PostprocessingVertexInput) -> EffectVertexOutput {
    let instance = instance_data[input.instance_index];

    let positions = effect_position_data(instance, input.vertex_index);

    var output: EffectVertexOutput;
    output.position = float4(positions.xy, 0.0, 1.0);
    output.texture_coordinates = positions.zw;
    output.color = float4(instance.color0, instance.color1);
    return output;
}

[[shader("pixel")]]
func fs_main(input: EffectVertexOutput) -> float4 {
    return texture.Sample(linear_sampler, input.texture_coordinates) * input.color;
}
