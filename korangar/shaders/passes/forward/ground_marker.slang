#language slang 2026

import globals;

// Emissive, depth-tested ground quads: cast rings and similar markers that
// must be occluded by entities standing in front of them. Unlike the walk
// indicator these are unlit, since they represent glowing magic.

struct GroundMarkerVertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] var texture_coordinates: float2;
    [[vk::location(1)]] var color: float4;
    [[vk::location(2)]] var base_coordinates: float2;
    [[vk::location(3)]] var edge_fade: float;
};

struct GroundMarker {
    float4 corners[4];
    float4 color;
    // uv_scale.xy, uv_offset.zw
    float4 uv_transform;
    // edge_fade in x, rest padding.
    float4 parameters;
};

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 2)]] var markers: StructuredBuffer<GroundMarker>;
[[vk::binding(1, 2)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(uint vertex_index: SV_VulkanVertexID, uint instance_index: SV_VulkanInstanceID) -> GroundMarkerVertexOutput {
    let marker = markers[instance_index];
    let world_position = corner_data(marker, vertex_index);

    let base_coordinates = uv_data(vertex_index);

    var output: GroundMarkerVertexOutput;
    output.position = mul(global_uniforms.view_projection, world_position);
    output.texture_coordinates = base_coordinates * marker.uv_transform.xy + marker.uv_transform.zw;
    output.color = marker.color;
    output.base_coordinates = base_coordinates;
    output.edge_fade = marker.parameters.x;
    return output;
}

[[shader("pixel")]]
func fs_main(input: GroundMarkerVertexOutput) -> float4 {
    // Fading toward the horizontal edges hides the hard silhouette of quads
    // faking a curved surface, and reads as the curvature itself: a glowing
    // cylinder seen face-on is brightest at its centre.
    let edge = sin(3.14159265 * input.base_coordinates.x);
    let fade = lerp(1.0, edge, input.edge_fade);
    return texture.Sample(linear_sampler, input.texture_coordinates) * input.color * fade;
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

// Standard orientation: U runs left to right, V top to bottom, matching the
// corner order. The walk indicator's table is transposed relative to this,
// which its rotationally symmetric texture hides; the scrolling cone bands
// do not.
[ForceInline]
func uv_data(vertex_index: uint) -> float2 {
    switch (vertex_index) {
        case 0:
            return float2(0.0, 0.0);
        case 1:
            return float2(1.0, 0.0);
        case 2:
            return float2(0.0, 1.0);
        case 3:
            return float2(0.0, 1.0);
        case 4:
            return float2(1.0, 0.0);
        default:
            return float2(1.0, 1.0);
    }
}
