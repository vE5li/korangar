#language slang 2026

import point_shadow;
import transform;

struct EntityVertexOutput {
    float4 position: SV_Position;
    [[vk::location(0)]] var world_position: float4;
    [[vk::location(1)]] var texture_coordinates: float2;
    [[vk::location(2)]] var depth_offset: float;
    [[vk::location(3)]] var curvature: float;
    [[vk::location(4)]] var original_depth_offset: float;
    [[vk::location(5)]] var original_curvature: float;
    [[vk::location(6)]] var alpha: float;
};

struct EntityFragmentOutput {
    float depth: SV_Depth;
};

[[vk::binding(1, 0)]] var nearest_sampler: SamplerState;
[[vk::binding(0, 1)]] var pass_uniforms: ConstantBuffer<PassUniforms>;
[[vk::binding(0, 2)]] var instance_data: StructuredBuffer<EntityInstanceData>;
[[vk::binding(0, 3)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(input: EntityVertexInput) -> EntityVertexOutput {
    let instance = instance_data[input.instance_index];
    let vertex = entity_vertex_data(input.vertex_index);
    let frame_part_vertex = mul(instance.frame_part_transform, float4(vertex.position, 1.0));

    var output: EntityVertexOutput;
    output.world_position = mul(instance.world, frame_part_vertex);
    output.position = mul(pass_uniforms.view_projection, output.world_position);
    output.texture_coordinates = instance.texture_position + vertex.texture_coordinates * instance.texture_size;

    if (instance.mirror != 0) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    // Values are represented as proportions ranging from -1 to 1
    let proportion_x = instance.frame_size.x / SPRITE_MAX_SIZE_X;
    let proportion_y = instance.frame_size.y / SPRITE_MAX_SIZE_Y;

    // The depth multiplier and curvature multiplier is derived from the truth table of vertex_data
    // Because we have to transform the vertex of the frame part, we can't use the depth and curvature
    // directly and are using the fact, that y / depth and x / curvature correlate to each other.
    // An offset is also added for frame parts not stay at the same depth.
    output.depth_offset = frame_part_vertex.y * proportion_y + instance.extra_depth_offset;
    output.curvature = frame_part_vertex.x * proportion_x;

    output.original_depth_offset = instance.depth_offset;
    output.original_curvature = instance.curvature;
    output.alpha = instance.alpha;
    return output;
}

[[shader("pixel")]]
func fs_main(input: EntityVertexOutput) -> EntityFragmentOutput {
    let diffuse_color = texture.Sample(nearest_sampler, input.texture_coordinates);
    if (diffuse_color.a != 1.0 || input.alpha != 1.0) {
        discard;
    }

    // Adjust the sprite as if it was standing upright
    let depth_offset = input.depth_offset * input.original_depth_offset;
    let curvature_offset = (0.5 - pow(input.curvature, 2.0)) * input.original_curvature;
    let view_position = mul(pass_uniforms.view, input.world_position);
    let adjusted_view_position = view_position - float4(0.0, 0.0, depth_offset + curvature_offset, 0.0);
    let adjusted_world_position = mul(pass_uniforms.inverse_view, adjusted_view_position);
    let light_distance = length(adjusted_world_position.xyz - pass_uniforms.light_position.xyz);

    var output: EntityFragmentOutput;
    output.depth = depth.linearToNonLinear(light_distance);
    return output;
}
