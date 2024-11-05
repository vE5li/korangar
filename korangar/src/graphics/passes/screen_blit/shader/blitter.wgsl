@group(0) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let color = textureLoad(texture, vec2<i32>(position.xy), 0);
    let srgb = linear_to_srgb_vec3(color.rgb);
    return vec4<f32>(srgb.rgb, color.a);
}

fn linear_to_srgb_vec3(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_srgb(color.x),
        linear_to_srgb(color.y),
        linear_to_srgb(color.z)
    );
}

fn linear_to_srgb(value: f32) -> f32 {
    let is_linear = value < 0.0031308;
    let linear_result = value * 12.92;
    let power_result = 1.055 * pow(abs(value), 1.0 / 2.4) - 0.055;
    return select(power_result, linear_result, is_linear);
}
