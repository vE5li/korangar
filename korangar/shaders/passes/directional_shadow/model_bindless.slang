#language slang 2026

import directional_shadow;

struct ModelVertexInput {
	uint instance_id : SV_VulkanInstanceID;
	[[vk::location(0)]] var position: float3;
	[[vk::location(2)]] var texture_coordinates: float2;
	[[vk::location(4)]] var texture_index: int;
	[[vk::location(5)]] var wind_affinity: float;
};

struct ModelVertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] var texture_coordinates: float2;
    [[vk::location(1)]] var texture_index: int;
};

struct ModelFragmentOutput {
    [[vk::location(0)]] var translucence: float;
};

[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var pass_uniforms: ConstantBuffer<PassUniforms>;
[[vk::binding(0, 2)]] var instance_data: StructuredBuffer<ModelInstanceData>;
[[vk::binding(0, 3)]] var textures: Texture2D[];

[[shader("vertex")]]
func vs_main(input: ModelVertexInput) -> ModelVertexOutput {
    let instance = instance_data[input.instance_id];

    let world_position = mul(instance.world, float4(input.position, 1.0));
    let wind_position = world_position + float4(pass_uniforms.animation_timer);
    let offset = float4(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * input.wind_affinity;

    var output: ModelVertexOutput;
    output.position = mul(pass_uniforms.view_projection, (world_position + offset));
    output.texture_coordinates = input.texture_coordinates;
    output.texture_index = input.texture_index;
    return output;
}

[[shader("pixel")]]
func fs_main(input: ModelVertexOutput) -> ModelFragmentOutput {
    let texture = textures[input.texture_index];
    let diffuse_color = texture.SampleLevel(linear_sampler, input.texture_coordinates, 0.0);

    if (diffuse_color.a == 0.0) {
        discard;
    }

    var output: ModelFragmentOutput;
    output.translucence = 1.0;
    return output;
}
