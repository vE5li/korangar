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
@group(1) @binding(0) var texture: texture_2d<u32>;
@group(1) @binding(1) var<storage, read_write> buffer: array<u32>;

@compute
@workgroup_size(1)
fn cs_main() {
    let texel_value = textureLoad(texture, global_uniforms.pointer_position, 0).rg;
    buffer[0] = texel_value.r;
    buffer[1] = texel_value.g;
}
