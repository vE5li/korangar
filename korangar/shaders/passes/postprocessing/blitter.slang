#language slang 2026

import screen_space;

[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var texture: Texture2D;

[[vk::constant_id(0)]] const var LUMA_IN_ALPHA: bool;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex.new(vertex_index);
}

[[shader("pixel")]]
func fs_main(input: FullscreenVertex) -> float4 {
    var color = texture.Sample(linear_sampler, input.uv);

    if (LUMA_IN_ALPHA) {
        /// Rec. 601 luma calculation for LDR sources.
        color.a = sqrt(dot(color.rgb, float3(0.299, 0.587, 0.114)));
    }

    return color;
}
