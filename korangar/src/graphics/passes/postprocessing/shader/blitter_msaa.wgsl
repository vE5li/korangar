override SAMPLE_COUNT: i32;
override LUMA_IN_ALPHA: bool;

@group(1) @binding(0) var texture: texture_multisampled_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    var output: VertexOutput;
    output.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    output.uv = uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texture_size = vec2<f32>(textureDimensions(texture));
    let uv = input.uv * texture_size;

    var color = vec4<f32>(0.0);

    for (var sample_id: i32 = 0; sample_id < SAMPLE_COUNT; sample_id++) {
        color += textureLoad(texture, vec2<i32>(uv), sample_id);
    }

    color /= f32(SAMPLE_COUNT);

    if (LUMA_IN_ALPHA) {
        /// Rec. 601 luma calculation for LDR sources.
        color.a = sqrt(dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114)));
    }

    return color;
}
