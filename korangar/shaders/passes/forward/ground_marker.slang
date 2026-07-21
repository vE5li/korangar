#language slang 2026

import globals;

// Emissive, depth-tested ground quads: cast rings and similar markers that
// must be occluded by entities standing in front of them. Unlike the walk
// indicator these are unlit, since they represent glowing magic.

struct GroundMarkerVertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] var texture_coordinates: float2;
    [[vk::location(1)]] var color: float4;
};

struct GroundMarker {
    float4 corners[4];
    float4 color;
    // uv_scale.xy, uv_offset.zw
    float4 uv_transform;
};

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 2)]] var markers: StructuredBuffer<GroundMarker>;
[[vk::binding(1, 2)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(uint vertex_index: SV_VulkanVertexID, uint instance_index: SV_VulkanInstanceID) -> GroundMarkerVertexOutput {
    let marker = markers[instance_index];
    let world_position = corner_data(marker, vertex_index);

    var output: GroundMarkerVertexOutput;
    output.position = mul(global_uniforms.view_projection, world_position);
    output.texture_coordinates = uv_data(vertex_index) * marker.uv_transform.xy + marker.uv_transform.zw;
    output.color = marker.color;
    return output;
}

[[shader("pixel")]]
func fs_main(input: GroundMarkerVertexOutput) -> float4 {
    return texture.Sample(linear_sampler, input.texture_coordinates) * input.color;
}

// Corner order: 0 upper left, 1 upper right, 2 lower left, 3 lower right.
[ForceInline]
func corner_data(marker: GroundMarker, vertex_index: uint) -> float4 {
    switch (vertex_index) {
        case 0:
            return marker.corners[0];
        case 1:
            return marker.corners[1];
        case 2:
            return marker.corners[2];
        case 3:
            return marker.corners[2];
        case 4:
            return marker.corners[1];
        default:
            return marker.corners[3];
    }
}

// Same optimized truth table as the walk indicator.
[ForceInline]
func uv_data(vertex_index: uint) -> float2 {
    let index = 1 << vertex_index;
    let x = float((index & 0x2C) != 0);
    let y = float((index & 0x32) != 0);
    return float2(x, y);
}
