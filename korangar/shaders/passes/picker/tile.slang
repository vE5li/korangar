#language slang 2026

import globals;
import matrix;

struct VertexInput {
    [[vk::location(0)]] var position: float3;
    [[vk::location(1)]] var identifier: uint;
}

struct VertexOutput {
    float4 position: SV_Position;
    [[vk::location(0)]] var identifier: uint;
}

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;

[[vk::constant_id(0)]] const var tile_enum_value: float;

[[shader("vertex")]]
func vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = mul(global_uniforms.view_projection, float4(input.position, 1.0));
    output.identifier = input.identifier;
    return output;
}

[[shader("pixel")]]
func fs_main(input: VertexOutput) -> uint2 {
    return uint2(input.identifier, uint(tile_enum_value));
}
