#language slang 2026

import screen_space;

[[vk::binding(0, 0)]] var texture: Texture2D;

// A 6x6 Lanczos 3 kernel.
static const var KERNEL: float[36] = {
    0.00059812, -0.00332290, 0.01495304, 0.01495304, -0.00332290, 0.00059812,
    -0.00332290, 0.01846054, -0.08307242, -0.08307242, 0.01846054, -0.00332290,
    0.01495304, -0.08307242, 0.37382591, 0.37382591, -0.08307242, 0.01495304,
    0.01495304, -0.08307242, 0.37382591, 0.37382591, -0.08307242, 0.01495304,
    -0.00332290, 0.01846054, -0.08307242, -0.08307242, 0.01846054, -0.00332290,
    0.00059812, -0.00332290, 0.01495304, 0.01495304, -0.00332290, 0.00059812,
};

static const var KERNEL_SIZE: int = 6;
static const var BORDER_SIZE: int = 3;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex::new(vertex_index);
}

[[shader("pixel")]]
func fs_main(input: FullscreenVertex) -> float4 {
    var texture_dimensions: uint2;
    texture.GetDimensions(texture_dimensions.x, texture_dimensions.y);
    let max_position = int2(texture_dimensions.xy) - int2(1);

    let pixel_coords = int2(input.uv * int2(texture_dimensions.xy));

    var color = float4(0.0);

    for(int ky = 0; ky < KERNEL_SIZE; ky++) {
        for(int kx = 0; kx < KERNEL_SIZE; kx++) {
            let sample_position = int2(
                pixel_coords.x + kx - BORDER_SIZE,
                pixel_coords.y + ky - BORDER_SIZE,
            );
            let clamped_position = clamp(sample_position, int2(0), max_position);
            let source_color = texture.Load(int3(clamped_position, 0));
            color += source_color * KERNEL[ky * 6 + kx];
        }
    }

    return color;
}
