override LUMA_IN_ALPHA: bool;

@group(1) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    var color = textureLoad(texture, vec2<i32>(position.xy), 0);

    if (LUMA_IN_ALPHA) {
        /// Rec. 601 luma calculation for LDR sources.
        color.a = sqrt(dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114)));
    }

    return color;
}
