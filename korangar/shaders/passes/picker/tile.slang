#language slang 2026

import globals;
import matrix;

struct TileVertexInput {
    [[vk::location(0)]] var position: float3;
    [[vk::location(1)]] var identifier: uint;
}

struct TileVertexOutput {
    float4 position: SV_Position;
    [[vk::location(0)]] var identifier: uint;
}

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;

[[vk::constant_id(0)]] const var TILE_ENUM_VALUE: float;

[[shader("vertex")]]
func vs_main(input: TileVertexInput) -> TileVertexOutput {
    var output: TileVertexOutput;
    output.position = mul(global_uniforms.view_projection, float4(input.position, 1.0));
    output.identifier = input.identifier;
    return output;
}

[[shader("pixel")]]
func fs_main(input: TileVertexOutput) -> uint2 {
    return uint2(input.identifier, uint(TILE_ENUM_VALUE));
}
