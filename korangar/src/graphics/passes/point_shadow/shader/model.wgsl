struct PassUniforms {
    view_projection: mat4x4<f32>,
    light_position: vec4<f32>,
    animation_timer: f32
}

struct InstanceData {
    world: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) texture_index: i32,
}

@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var<uniform> pass_uniforms: PassUniforms;
@group(2) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(3) @binding(0) var textures: binding_array<texture_2d<f32>>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(3) texture_index: i32,
    @location(5) wind_affinity: f32,
    @location(6) instance_id: u32,
) -> VertexOutput {
    let instance = instance_data[instance_id];

    let world_position = instance.world * vec4<f32>(position, 1.0);
    let wind_position = world_position + vec4<f32>(pass_uniforms.animation_timer);
    let offset = vec4<f32>(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * wind_affinity;

    var output: VertexOutput;
    output.world_position = (world_position + offset);
    output.position = pass_uniforms.view_projection * output.world_position;
    output.texture_coordinates = texture_coordinates;
    output.texture_index = texture_index;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @builtin(frag_depth) f32 {
    var diffuse_color = textureSample(textures[input.texture_index], nearest_sampler, input.texture_coordinates);

    let light_distance = length(input.world_position.xyz - pass_uniforms.light_position.xyz);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    return light_distance / 256.0;
}
