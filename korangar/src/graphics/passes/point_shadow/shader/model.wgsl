struct PassUniforms {
    view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
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
}

@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var<uniform> pass_uniforms: PassUniforms;
@group(2) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(3) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(4) wind_affinity: f32,
    @location(5) instance_id: u32,
) -> VertexOutput {
    let instance = instance_data[instance_id];

    let world_position = instance.world * vec4<f32>(position, 1.0);
    let wind_position = world_position + vec4<f32>(pass_uniforms.animation_timer);
    let offset = vec4<f32>(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * wind_affinity;

    var output: VertexOutput;
    output.world_position = (world_position + offset);
    output.position = pass_uniforms.view_projection * output.world_position;
    output.texture_coordinates = texture_coordinates;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @builtin(frag_depth) f32 {
    var diffuse_color = textureSample(texture, nearest_sampler, input.texture_coordinates);

    let light_distance = length(input.world_position.xyz - pass_uniforms.light_position.xyz);

    if (diffuse_color.a == 0.0) {
        discard;
    }

    return linearToNonLinear(light_distance);
}

fn linearToNonLinear(linear_depth: f32) -> f32 {
    const NEAR_PLANE = 0.1;
    return NEAR_PLANE / (linear_depth + 1e-7);
}
