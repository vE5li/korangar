struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    screen_size: vec2<u32>,
    pointer_position: vec2<u32>,
    animation_timer: f32,
    day_timer: f32,
    water_level: f32,
    point_light_count: u32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(1) @binding(0) var diffuse_buffer: texture_multisampled_2d<f32>;

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
        blended += textureLoad(diffuse_buffer, pixel_coord, sample_id) * global_uniforms.ambient_color;
    }
    return blended / 4.0;
}
