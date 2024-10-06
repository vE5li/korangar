struct Matrices {
    view_projection: mat4x4<f32>,
}

struct Constants {
    world: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) texture_index: i32,
}

@group(0) @binding(0) var<uniform> matrices: Matrices;
@group(0) @binding(1) var texture_sampler: sampler;
@group(1) @binding(0) var textures: binding_array<texture_2d<f32>>;

var<push_constant> constants: Constants;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(3) texture_index: i32
) -> VertexOutput {
    var output: VertexOutput;
    output.position = matrices.view_projection * constants.world * vec4<f32>(position, 1.0);
    output.texture_coordinates = texture_coordinates;
    output.texture_index = texture_index;
    return output;
}

@fragment
fn fs_main(
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) texture_index: i32
) -> @location(0) vec2<u32> {
    let diffuse_color = textureSample(textures[texture_index], texture_sampler, texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    return vec2<u32>(0u);
}
