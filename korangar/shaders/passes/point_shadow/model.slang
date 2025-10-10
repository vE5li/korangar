#language slang 2026

import point_shadow;
import transform;

struct ModelVertexInput {
    uint instance_index : SV_VulkanInstanceID;
    [[vk::location(0)]] var position: float3;
    [[vk::location(2)]] var texture_coordinates: float2;
    [[vk::location(5)]] var wind_affinity: float;
};

struct ModelVertexOutput {
    float4 position: SV_Position;
    [[vk::location(0)]] var world_position: float4;
    [[vk::location(1)]] var texture_coordinates: float2;
};

[[vk::binding(1, 0)]] var nearest_sampler: SamplerState;
[[vk::binding(0, 1)]] var pass_uniforms: ConstantBuffer<PassUniforms>;
[[vk::binding(0, 2)]] var instance_data: StructuredBuffer<ModelInstanceData>;
[[vk::binding(0, 3)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(input: ModelVertexInput) -> ModelVertexOutput {
    let instance = instance_data[input.instance_index];

    let world_position = mul(instance.world, float4(input.position, 1.0));
    let wind_position = world_position + float4(pass_uniforms.animation_timer);
    let offset = float4(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * input.wind_affinity;

    var output: ModelVertexOutput;
    output.world_position = world_position + offset;
    output.position = mul(pass_uniforms.view_projection, output.world_position);
    output.texture_coordinates = input.texture_coordinates;
    return output;
}

[[shader("pixel")]]
func fs_main(input: ModelVertexOutput) -> ModelFragmentOutput {
    let diffuse_color = texture.Sample(nearest_sampler, input.texture_coordinates);

    let light_distance = length(input.world_position.xyz - pass_uniforms.light_position.xyz);

    if (diffuse_color.a == 0.0) {
        discard;
    }

    var output: ModelFragmentOutput;
    output.depth = depth.linearToNonLinear(light_distance);
    return output;
}
