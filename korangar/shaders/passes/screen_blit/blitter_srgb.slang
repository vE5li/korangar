#language slang 2026

import screen_space;
import globals;

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(0, 1)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex::new(vertex_index);
}

[[shader("pixel")]]
func fs_main(FullscreenVertex input) -> float4 {
    let linear_color = texture.Load(int3(int2(input.position.xy), 0));
    return float4(linear_color.rgb * global_uniforms.brightness, linear_color.a);
}
