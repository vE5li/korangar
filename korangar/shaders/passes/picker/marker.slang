#language slang 2026

import picker;
import transform;

struct MarkerVertexInput {
    uint vertex_index: SV_VulkanVertexID;
    uint instance_index: SV_VulkanInstanceID;
}

struct MarkerVertexOutput {
    float4 position: SV_Position;
    [[vk::location(0)]] var identifier_high: uint;
    [[vk::location(1)]] var identifier_low: uint;
}

public struct MarkerInstanceData {
    public var screen_position: float2;
    public var screen_size: float2;
    public var identifier_high: uint;
    public var identifier_low: uint;
}

[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<MarkerInstanceData>;

[[shader("vertex")]]
func vs_main(input: MarkerVertexInput) -> MarkerVertexOutput {
    let instance = instance_data[input.instance_index];
    let vertex = vertex_data(input.vertex_index);

    let clip_size = instance.screen_size * 2.0;
    let position = coordinate_space::screen_to_clip_space(instance.screen_position) + vertex.xy * clip_size;

    var output: MarkerVertexOutput;
    output.position = float4(position, 1.0, 1.0);
    output.identifier_high = instance.identifier_high;
    output.identifier_low = instance.identifier_low;

    return output;
}

[[shader("pixel")]]
func fs_main(input: MarkerVertexOutput) -> uint2 {
    return uint2(input.identifier_low, input.identifier_high);
}

// Optimized version of the following truth table:
//
// vertex_index  x  y
// 0             0  0
// 1             1  0
// 2             1 -1
// 3             1 -1
// 4             0 -1
// 5             0  0
//
// (x,y) are the vertex position
[ForceInline]
public func vertex_data(vertex_index: uint) -> float2 {
    let index = 1 << vertex_index;
    let x = float((index & 0x0E) != 0u);
    let y = float((index & 0x1C) != 0u);
    return float2(x, -y);
}
