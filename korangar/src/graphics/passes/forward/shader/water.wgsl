struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> VertexOutput {
    let adjusted_position = vec3<f32>(
        position.x,
        position.y + sin(global_uniforms.animation_timer + position.x + position.z),
        position.z
    );
    let world_position = vec4<f32>(adjusted_position, 1.0);

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * world_position;
    output.world_position = world_position;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let delta = global_uniforms.water_level - input.world_position.y;
    let factor = max(0.2, delta / 30.0);
    let color = vec3<f32>(factor, min(factor / 2.0, 0.3), min(factor / 10.0, 0.1));
    return vec4<f32>(color, 1.0);
}
