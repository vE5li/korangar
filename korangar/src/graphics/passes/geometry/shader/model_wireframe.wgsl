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

struct InstanceData {
    world: mat4x4<f32>,
    inv_world: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec4<f32>,
}

struct FragmentOutput {
    @location(0) fragment_color: vec4<f32>,
    @location(1) fragment_normal: vec4<f32>,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(4) wind_affinity: f32,
    @location(5) instance_id: u32
) -> VertexOutput {
    let instance = instance_data[instance_id];

    let world_position = instance.world * vec4<f32>(position, 1.0);
    let wind_position = world_position + vec4<f32>(global_uniforms.animation_timer);
    let offset = vec4<f32>(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * wind_affinity;

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * (world_position + offset);
    output.normal = instance.inv_world * vec4<f32>(normal, 1.0);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    var output: FragmentOutput;
    output.fragment_color = vec4<f32>(1.0);
    output.fragment_normal = input.normal;
    return output;
}
