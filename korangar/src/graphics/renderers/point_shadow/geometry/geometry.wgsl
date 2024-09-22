struct Constants {
    world: mat4x4<f32>,
    // The w component of the light_position is actually the time. This is a small optimization.
    light_position: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) @interpolate(flat) texture_index: i32,
}

struct FragmentInput {
    @location(0) world_position: vec4<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) @interpolate(flat) texture_index: i32,
}

@group(0) @binding(0) var texture_sampler: sampler;
@group(1) @binding(0) var textures: binding_array<texture_2d<f32>>;
@group(2) @binding(0) var<uniform> view_projection: mat4x4<f32>;

var<push_constant> constants: Constants;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(3) texture_index: i32,
    @location(4) wind_affinity: f32,
) -> VertexOutput {
    let world_position = constants.world * vec4<f32>(position, 1.0);
    let wind_position = world_position + vec4<f32>(constants.light_position.w);
    let offset = vec4<f32>(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * wind_affinity;

    var output: VertexOutput;
    output.world_position = (world_position + offset);
    output.position = view_projection * output.world_position;
    output.texture_coordinates = texture_coordinates;
    output.texture_index = texture_index;
    return output;
}

@fragment
fn fs_main(fragment: FragmentInput) -> @builtin(frag_depth) f32 {
    var diffuse_color = textureSample(textures[fragment.texture_index], texture_sampler, fragment.texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    let light_distance = length(fragment.world_position.xyz - constants.light_position.xyz);

    return light_distance / 256.0;
}
