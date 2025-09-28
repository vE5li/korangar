#language slang 2026

import globals;
import matrix;
import postprocessing;

public struct DebugAABBVertexInput {
    [[vk::location(0)]] public var position: float3;
    public uint instance_index : SV_VulkanInstanceID;
}

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<DebugAABBInstanceData>;

[[shader("vertex")]]
func vs_main(input: DebugAABBVertexInput) -> DebugVertexOutput {
    let instance = instance_data[input.instance_index];

    var output: DebugVertexOutput;
    output.position = mul(global_uniforms.view_projection, mul(instance.world, float4(input.position, 1.0)));
    output.color = instance.color;
    return output;
}

[[shader("pixel")]]
func fs_main(input: DebugVertexOutput) -> float4 {
    return input.color;
}
