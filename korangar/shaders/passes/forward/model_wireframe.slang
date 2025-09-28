#language slang 2026

import globals;
import forward;
import matrix;

struct WireframeVertexInput {
    uint instance_id: SV_VulkanInstanceID;
    [[vk::location(0)]] float3 position;
    [[vk::location(1)]] float3 normal;
    [[vk::location(5)]] float wind_affinity;
}

struct WireframeVertexOutput {
    float4 position: SV_Position;
    [[vk::location(0)]] var normal: float4;
}

public struct WaveFragmentOutput {
    [[vk::location(0)]] public var fragment_color: float4;
    [[vk::location(1)]] public var fragment_normal: float4;
}

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(0, 2)]] var instance_data: StructuredBuffer<ModelInstanceData>;

[[shader("vertex")]]
func vs_main(input: WireframeVertexInput) -> WireframeVertexOutput {
    let instance = instance_data[input.instance_id];

    let world_position = mul(instance.world, float4(input.position, 1.0));
    let wind_position = world_position + float4(global_uniforms.animation_timer);
    let offset = float4(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * input.wind_affinity;

    var output: WireframeVertexOutput;
    output.position = mul(global_uniforms.view_projection, world_position + offset);
    output.normal = mul(instance.inv_world, float4(input.normal, 1.0));
    return output;
}

[[shader("pixel")]]
func opaque_main(input: WireframeVertexOutput) -> WaveFragmentOutput {
    var output: WaveFragmentOutput;
    output.fragment_color = float4(1.0);
    output.fragment_normal = input.normal;
    return output;
}
