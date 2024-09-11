struct Matrices {
    view_projection: mat4x4<f32>,
    time: f32,
}

struct Constants {
    world: mat4x4<f32>,
    inv_world: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec4<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) @interpolate(flat) texture_index: i32,
}

struct FragmentInput {
    @location(0) normal: vec4<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) @interpolate(flat) texture_index: i32,
}

struct FragmentOutput {
    @location(0) fragment_color: vec4<f32>,
    @location(1) fragment_normal: vec4<f32>,
}

@group(0) @binding(0) var<uniform> matrices: Matrices;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var textures: binding_array<texture_2d<f32>>;

var<push_constant> constants: Constants;
override additional_color: f32;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(3) texture_index: i32,
    @location(4) wind_affinity: f32,
) -> VertexOutput {
    let world_position = constants.world * vec4<f32>(position, 1.0);
    let wind_position = world_position + vec4<f32>(matrices.time);
    let offset = vec4<f32>(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * wind_affinity;

    var output: VertexOutput;
    output.position = matrices.view_projection * (world_position + offset);
    output.normal = constants.inv_world * vec4<f32>(normal, 1.0);
    output.texture_coordinates = texture_coordinates;
    output.texture_index = texture_index;
    return output;
}

@fragment
fn fs_main(fragment: FragmentInput) -> FragmentOutput {

    let diffuse_color = textureSample(textures[fragment.texture_index], linear_sampler, fragment.texture_coordinates);
    let alpha_channel = textureSample(textures[fragment.texture_index], nearest_sampler, fragment.texture_coordinates).a;

    if (alpha_channel + additional_color < 1.0) {
        discard;
    }

    var output: FragmentOutput;
    output.fragment_color = vec4<f32>(diffuse_color.rgb + additional_color, diffuse_color.a);
    output.fragment_normal = fragment.normal;
    return output;
}
