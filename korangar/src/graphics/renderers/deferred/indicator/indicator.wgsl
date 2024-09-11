struct Matrices {
    view_projection: mat4x4<f32>,
}

struct Constants {
    upper_left: vec4<f32>,
    upper_right: vec4<f32>,
    lower_left: vec4<f32>,
    lower_right: vec4<f32>,
    color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) normal: vec4<f32>,
}

struct FragmentOutput {
    @location(0) fragment_color: vec4<f32>,
    @location(1) fragment_normal: vec4<f32>,
}

@group(0) @binding(0) var<uniform> matrices: Matrices;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var texture: texture_2d<f32>;

var<push_constant> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    output.position = matrices.view_projection * vertex_data(vertex_index);
    output.texture_coordinates = uv_data(vertex_index);
    output.normal = vec4<f32>(normal_data(vertex_index), 1.0);
    return output;
}

@fragment
fn fs_main(
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) normal: vec4<f32>,
) -> FragmentOutput {
    let texel_color = textureSample(texture, nearest_sampler, texture_coordinates);

    if (texel_color.a < 0.1) {
        discard;
    }

    var output: FragmentOutput;
    output.fragment_color = vec4<f32>(texel_color.rgb * constants.color.rgb, texel_color.a);
    output.fragment_normal = normal;
    return output;
}

fn vertex_data(vertex_index: u32) -> vec4<f32> {
    switch (vertex_index) {
        case 0u: {
            return constants.upper_left;
        }
        case 1u: {
            return constants.upper_right;
        }
        case 2u: {
            return constants.lower_left;
        }
        case 3u: {
            return constants.lower_left;
        }
        case 4u: {
            return constants.upper_right;
        }
        default: {
            return constants.lower_right;
        }
    }
}

// Optimized version of the following truth table:
//
// vertex_index  x  y
// 0             0  0
// 1             0  1
// 2             1  0
// 3             1  0
// 4             0  1
// 5             1  1
//
// (x,y) are the UV coordinates
fn uv_data(vertex_index: u32) -> vec2<f32> {
    let index = 1u << vertex_index;
    let x = f32((index & 0x2Cu) != 0u);
    let y = f32((index & 0x32u) != 0u);
    return vec2<f32>(x, y);
}

fn normal_data(vertex_index: u32) -> vec3<f32> {
    if (vertex_index < 3) {
        return normalize(cross(
            (constants.upper_right - constants.upper_left).xyz,
            (constants.lower_left - constants.upper_left).xyz
        ));
    } else {
        return normalize(cross(
            (constants.upper_right - constants.lower_left).xyz,
            (constants.lower_right - constants.lower_left).xyz
        ));
    }
}
