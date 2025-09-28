#language slang 2026

import screen_space;

[[vk::binding(0, 1)]] var accumulation_texture: Texture2D;
[[vk::binding(0, 2)]] var revealage_texture: Texture2D;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex.new(vertex_index);
}

[[shader("pixel")]]
func fs_main(input: FullscreenVertex) -> float4 {
    let pixel_coord = int2(input.position.xy);
    let accumulation = accumulation_texture.Load(int3(pixel_coord, 0));
    let revealage = revealage_texture.Load(int3(pixel_coord, 0)).r;
    let color = float4(accumulation.rgb / max(accumulation.a, 1e-5), revealage);

    if (color.a > 0.99) {
        discard;
    }

    return color;
}
