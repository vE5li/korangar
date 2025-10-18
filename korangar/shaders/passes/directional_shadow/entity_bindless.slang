#language slang 2026

import directional_shadow;

struct EntityVertexOutput {
    float4 position : SV_Position;
    [[vk::location(0)]] var world_position: float4;
    [[vk::location(1)]] var texture_coordinates: float2;
    [[vk::location(2)]] var depth_offset: float;
    [[vk::location(3)]] var curvature: float;
    [[vk::location(4)]] var original_depth_offset: float;
    [[vk::location(5)]] var original_curvature: float;
    [[vk::location(6)]] var texture_index: int;
    [[vk::location(7)]] var alpha: float;
};

[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var pass_uniforms: ConstantBuffer<PassUniforms>;
[[vk::binding(0, 2)]] var instance_data: StructuredBuffer<EntityInstanceData>;
[[vk::binding(1, 2)]] var textures: Texture2D[];

[[shader("vertex")]]
func vs_main(input: EntityVertexInput) -> EntityVertexOutput {
    let instance = instance_data[input.instance_index];
    let vertex = entity_vertex_data(input.vertex_index);
    let frame_part_vertex = mul(instance.frame_part_transform, float4(vertex.position, 1.0));
    let world_position = mul(instance.world, frame_part_vertex);

    var output: EntityVertexOutput;
    output.world_position = world_position;
    output.position = mul(pass_uniforms.view_projection, world_position);
    output.texture_coordinates = instance.texture_position + vertex.texture_coordinates * instance.texture_size;

    if (instance.mirror != 0) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    // Values are represented as proportions ranging from -1 to 1.
    let proportion_x = instance.frame_size.x / SPRITE_MAX_SIZE_X;
    let proportion_y = instance.frame_size.y / SPRITE_MAX_SIZE_Y;

    // The depth multiplier and curvature multiplier is derived from the truth
    // table of entity_vertex_data. Because we have to transform the vertex of the
    // frame part, we can't use the depth and curvature directly and are using the
    // fact, that y / depth and x / curvature correlate to each other. An offset
    // is also added for frame parts not stay at the same depth.
    output.depth_offset = frame_part_vertex.y * proportion_y + instance.extra_depth_offset;
    output.curvature = frame_part_vertex.x * proportion_x;

    output.original_depth_offset = instance.depth_offset;
    output.original_curvature = instance.curvature;
    output.texture_index = instance.texture_index;
    output.alpha = instance.alpha;
    return output;
}

[[shader("pixel")]]
func fs_main_opaque(input: EntityVertexOutput) -> EntityFragmentOutput {
    let texture = textures[input.texture_index];
    let diffuse_color = texture.SampleLevel(linear_sampler, input.texture_coordinates, 0.0);

    if (diffuse_color.a != 1.0 || input.alpha != 1.0) {
        discard;
    }

    let clip_position = calculate_adjusted_clip_position(
        input.world_position,
        input.depth_offset,
        input.curvature,
        input.original_depth_offset,
        input.original_curvature,
        pass_uniforms
    );

    // Fully opaque, so combined_alpha is 1.0
    return create_shadow_output(1.0, clip_position);
}

[[shader("pixel")]]
func fs_main_transparent(input: EntityVertexOutput) -> EntityFragmentOutput {
    let texture = textures[input.texture_index];
    let diffuse_color = texture.SampleLevel(linear_sampler, input.texture_coordinates, 0.0);

    if (diffuse_color.a != 1.0 || input.alpha < 0.05) {
        discard;
    }

    let clip_position = calculate_adjusted_clip_position(
        input.world_position,
        input.depth_offset,
        input.curvature,
        input.original_depth_offset,
        input.original_curvature,
        pass_uniforms
    );

    return create_shadow_output(diffuse_color.a * input.alpha, clip_position);
}
