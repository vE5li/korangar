#language slang 2026

import screen_space;

[[vk::binding(0, 0)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex::new(vertex_index);
}

[[shader("pixel")]]
func fs_main(FullscreenVertex input) -> float4 {
    return texture.Load(int3(int2(input.position.xy), 0));
}
