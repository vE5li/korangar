#language slang 2026

import globals;
import matrix;
import postprocessing;

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<DebugRectangleInstanceData>;

[[shader("vertex")]]
func vs_main(input: DebugRectangleVertexInput) -> DebugVertexOutput {
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
