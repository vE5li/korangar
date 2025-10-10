#language slang 2026

import globals;
import matrix;
import picker;
import transform;

struct EntityVertexInput {
    uint vertex_index: SV_VulkanVertexID;
    uint instance_index: SV_VulkanInstanceID;
}

struct EntityVertexOutput {
    float4 position: SV_Position;
    [[vk::location(0)]] var texture_coordinates: float2;
    [[vk::location(1)]] var texture_index: int;
    [[vk::location(2)]] var identifier_high: uint;
    [[vk::location(3)]] var identifier_low: uint;
}

struct EntityFragmentOutput {
    float depth: SV_Depth;
    [[vk::location(0)]] var identifier: uint2;
}

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(1, 0)]] var nearest_sampler: SamplerState;
[[vk::binding(0, 1)]] var instance_data: StructuredBuffer<EntityInstanceData>;
[[vk::binding(1, 1)]] var textures: Texture2D[];

[[shader("vertex")]]
func vs_main(input: EntityVertexInput) -> EntityVertexOutput {
    let instance = instance_data[input.instance_index];
    let vertex = entity_vertex_data(input.vertex_index);
    let frame_part_vertex = mul(instance.frame_part_transform, float4(vertex.position, 1.0));

    var output: EntityVertexOutput;
    output.position = mul(global_uniforms.view_projection, mul(instance.world, frame_part_vertex));
    output.texture_coordinates = instance.texture_position + vertex.texture_coordinates * instance.texture_size;
    output.texture_index = instance.texture_index;

    if (instance.mirror != 0) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    output.identifier_high = instance.identifier_high;
    output.identifier_low = instance.identifier_low;
    return output;
}

[[shader("pixel")]]
func fs_main(input: EntityVertexOutput) -> EntityFragmentOutput {
    let diffuse_color = textures[input.texture_index].Sample(nearest_sampler, input.texture_coordinates);
    if (diffuse_color.a != 1.0) {
        discard;
    }

    // We add a small offset in linear space, so that entities don't clip into the ground.
    let linear_z = depth::nonLinearToLinear(input.position.z);
    let adjusted_linear_z = linear_z - 2.0;
    let non_linear_z = depth::linearToNonLinear(adjusted_linear_z);
    let clamped_depth = saturate(non_linear_z);

    var output: EntityFragmentOutput;
    output.identifier = uint2(input.identifier_low, input.identifier_high);
    output.depth = clamped_depth;
    return output;
}
