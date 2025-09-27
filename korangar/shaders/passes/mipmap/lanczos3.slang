#language slang 2026

import screen_space;

[[vk::binding(0, 0)]] var source_texture: Texture2D;

// A 6x6 Lanczos 3 kernel.
static const var KERNEL: float[36] = {
    0.00059812, -0.00332290, 0.01495304, 0.01495304, -0.00332290, 0.00059812,
    -0.00332290, 0.01846054, -0.08307242, -0.08307242, 0.01846054, -0.00332290,
    0.01495304, -0.08307242, 0.37382591, 0.37382591, -0.08307242, 0.01495304,
    0.01495304, -0.08307242, 0.37382591, 0.37382591, -0.08307242, 0.01495304,
    -0.00332290, 0.01846054, -0.08307242, -0.08307242, 0.01846054, -0.00332290,
    0.00059812, -0.00332290, 0.01495304, 0.01495304, -0.00332290, 0.00059812,
};

static const var KERNEL_SIZE: uint = 6;
static const var BORDER_SIZE: int = 3;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex::new(vertex_index);
}

[[shader("pixel")]]
func fs_main(input: FullscreenVertex) -> float4 {
    var texture_dimensions: float2;
    source_texture.GetDimensions(texture_dimensions.x, texture_dimensions.y);
    let pixel_coords = int2(input.uv * float2(texture_dimensions.xy));

    var color = float4(0.0);

    for(uint ky = 0; ky < KERNEL_SIZE; ky++) {
        for(uint kx = 0; kx < KERNEL_SIZE; kx++) {
            let sample_position = int2(
                pixel_coords.x + int(kx) - BORDER_SIZE,
                pixel_coords.y + int(ky) - BORDER_SIZE
            );
            let clamped_position = clamp(sample_position, int2(0), int2(texture_dimensions.xy) - 1);
            let source_color = source_texture.Load(int3(clamped_position, 0));
            color += source_color * KERNEL[ky * 6u + kx];
        }
    }

    return color;
}
