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

struct InstanceData {
    world: mat4x4<f32>,
    inv_world: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec4<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) texture_index: i32,
    @location(3) color: vec3<f32>,
}

struct FragmentOutput {
    @location(0) fragment_color: vec4<f32>,
    @location(1) fragment_normal: vec4<f32>,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(3) var texture_sampler: sampler;
@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(2) @binding(0) var textures: binding_array<texture_2d<f32>>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(3) texture_index: i32,
    @location(4) color: vec3<f32>,
    @location(5) wind_affinity: f32,
    @location(6) instance_id: u32
) -> VertexOutput {
    let instance = instance_data[instance_id];

    let world_position = instance.world * vec4<f32>(position, 1.0);
    let wind_position = world_position + vec4<f32>(global_uniforms.animation_timer);
    let offset = vec4<f32>(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * wind_affinity;

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * (world_position + offset);
    output.normal = instance.inv_world * vec4<f32>(normal, 1.0);
    output.texture_coordinates = texture_coordinates;
    output.texture_index = texture_index;
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    let diffuse_color = textureSample(textures[input.texture_index], texture_sampler, input.texture_coordinates);
    let alpha_channel = textureSample(textures[input.texture_index], nearest_sampler, input.texture_coordinates).a;

    if (alpha_channel < 1.0) {
        discard;
    }

    var output: FragmentOutput;
    output.fragment_color = vec4<f32>(diffuse_color.rgb * input.color, 1.0);
    output.fragment_normal = input.normal;
    return output;
}
