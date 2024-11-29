struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinate: vec2<f32>,
}

@group(0) @binding(0) var source_texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    var output: VertexOutput;
    output.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    output.texture_coordinate = uv;
    return output;
}

// A 6x6 Lanczos 3 kernel.
const KERNEL: array<f32, 36> = array<f32, 36>(
    0.00059812, -0.00332290, 0.01495304, 0.01495304, -0.00332290, 0.00059812,
    -0.00332290, 0.01846054, -0.08307242, -0.08307242, 0.01846054, -0.00332290,
    0.01495304, -0.08307242, 0.37382591, 0.37382591, -0.08307242, 0.01495304,
    0.01495304, -0.08307242, 0.37382591, 0.37382591, -0.08307242, 0.01495304,
    -0.00332290, 0.01846054, -0.08307242, -0.08307242, 0.01846054, -0.00332290,
    0.00059812, -0.00332290, 0.01495304, 0.01495304, -0.00332290, 0.00059812,
);

const KERNEL_SIZE: u32 = 6;
const BORDER_SIZE: i32 = 3;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texture_dimensions = textureDimensions(source_texture);
    let pixel_coords = vec2<i32>(input.texture_coordinate * vec2<f32>(texture_dimensions));

    var color = vec4<f32>(0.0);

    for(var ky = 0u; ky < KERNEL_SIZE; ky++) {
        for(var kx = 0u; kx < KERNEL_SIZE; kx++) {
            let sample_position = vec2<i32>(
                pixel_coords.x + i32(kx) - BORDER_SIZE,
                pixel_coords.y + i32(ky) - BORDER_SIZE
            );
            let clamped_position = clamp(sample_position, vec2<i32>(0), vec2<i32>(texture_dimensions) - 1);
            let source_color = textureLoad(source_texture, clamped_position, 0);
            color += source_color * KERNEL[ky * 6u + kx];
        }
    }

    return color;
}
