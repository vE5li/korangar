@group(1) @binding(4) var interface_buffer: texture_multisampled_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel_coord = vec2<i32>(position.xy);
    var blended = vec4<f32>(0.0);
    for (var sample_id: i32 = 0; sample_id < 4; sample_id++) {
        blended += textureLoad(interface_buffer, pixel_coord, sample_id);
    }
    return blended / 4.0;
}
