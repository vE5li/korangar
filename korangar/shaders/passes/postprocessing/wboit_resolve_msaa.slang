#language slang 2026

import screen_space;

[[vk::binding(0, 1)]] var accumulation_texture: Texture2DMS;
[[vk::binding(0, 2)]] var revealage_texture: Texture2DMS;

[[vk::constant_id(0)]] const var MSAA_SAMPLE_COUNT: int;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex.new(vertex_index);
}

[[shader("pixel")]]
func fs_main(input: FullscreenVertex) -> float4 {
    let pixel_coord = int2(input.position.xy);
    var color = float4(0.0);

    [unroll]
    for (var index: int = 0; index < MSAA_SAMPLE_COUNT; index++) {
        let accumulation = accumulation_texture.Load(pixel_coord, index);
        let revealage = revealage_texture.Load(pixel_coord, index).r;
        color += float4(accumulation.rgb / max(accumulation.a, 1e-5), revealage);
    }

    color = color / float(MSAA_SAMPLE_COUNT);

    if (color.a > 0.99) {
        discard;
    }

    return color;
}
