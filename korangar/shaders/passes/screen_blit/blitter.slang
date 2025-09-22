#language slang 2026

import screen_space;
import transform;

[[vk::binding(0, 0)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex::new(vertex_index);
}

[[shader("pixel")]]
func fs_main(FullscreenVertex input) -> float4 {
    let linear_color = texture.Load(int3(int2(input.position.xy), 0));
    let srgb_color = color::linear_to_srgb(linear_color.rgb);
    return float4(srgb_color.rgb, linear_color.a);
}
