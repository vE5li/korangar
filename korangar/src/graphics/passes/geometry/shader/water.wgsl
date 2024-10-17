struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    view:mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    pointer_position: vec2<u32>,
    animation_timer: f32,
    day_timer: f32,
    water_level: f32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    let adjusted_position = vec3<f32>(
        position.x,
        position.y + sin(global_uniforms.animation_timer + position.x + position.z),
        position.z
    );
    return global_uniforms.view_projection * vec4<f32>(adjusted_position, 1.0);
}

@fragment
fn fs_main() -> @location(2) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 0.0);
}
