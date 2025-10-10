#language slang 2026

import postprocessing;

public struct DebugCircleVertexOutput {
    public float4 position : SV_Position;
    [[vk::location(0)]] public var fragment_position: float2;
    [[vk::location(1)]] public var instance_index: uint;
}

public struct DebugCircleInstanceData {
    public var position: float4;
    public var color: float4;
    public var screen_position: float2;
    public var screen_size: float2;
}

[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<DebugCircleInstanceData>;

[[shader("vertex")]]
func vs_main(input: PostprocessingVertexInput) -> DebugCircleVertexOutput {
    let instance = instance_data[input.instance_index];

    let vertex = vertex_data(input.vertex_index);
    let clip_size = instance.screen_size * 2.0;
    let clip_position = screen_to_clip_space(instance.screen_position) + vertex.xy * clip_size;

    var output: DebugCircleVertexOutput;
    output.position = float4(clip_position, 0.0, 1.0);
    output.fragment_position = float2(vertex);
    output.instance_index = input.instance_index;
    return output;
}

[[shader("pixel")]]
func fs_main(input: DebugCircleVertexOutput) -> float4 {
    let instance = instance_data[input.instance_index];

    let distance_from_center = distance(input.fragment_position.xy, float2(0.5, -0.5));
    let scaling_factor = instance.screen_size.y / 2.0;
    let intensity = clamp(gaussian_peak(distance_from_center, scaling_factor), 0.0, 1.0);

    return instance.color * intensity;
}

[ForceInline]
func gaussian_peak(x: float, scaling_factor: float) -> float {
    // The variance defines how steep the peek is.
    // A larger number will make it fuzzier, a smaller number will make the circle sharper.
    let variance = 0.0002 / scaling_factor;

    // The expected value defines the position of the peek.
    // Since x ranges from 0.0 at the center to 0.5 at the closest edge we want a little less than 0.5 to not cut off the edges.
    let expected_value = 0.5 - variance * 10.0;

    return exp(-(pow(x - expected_value, 2.0)) / (2.0 * variance * variance));
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
func vertex_data(vertex_index: uint) -> float2 {
    let index = 1u << vertex_index;
    let x = float((index & 0xEu) != 0u);
    let y = float((index & 0x1Cu) != 0u);
    return float2(x, -y);
}
