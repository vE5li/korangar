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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) identifier: u32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;

override tile_enum_value: f32;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) identifier: u32
) -> VertexOutput {
    var output: VertexOutput;
    output.position = global_uniforms.view_projection * vec4<f32>(position, 1.0);
    output.identifier = identifier;
    return output;
}

@fragment
fn fs_main(@location(0) identifier: u32) -> @location(0) vec2<u32> {
    return vec2<u32>(identifier, u32(tile_enum_value));
}
