struct Constants {
    view_projection: mat4x4<f32>,
    upper_left: vec4<f32>,
    upper_right: vec4<f32>,
    lower_left: vec4<f32>,
    lower_right: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
}

@group(0) @binding(0) var sampled_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;

var<push_constant> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    output.position = constants.view_projection * position_data(vertex_index);
    output.texture_coordinates = uv_data(vertex_index);
    return output;
}

@fragment
fn fs_main(@location(0) texture_coordinates: vec2<f32>) -> @location(0) vec4<f32> {
    let fragment_color = textureSample(sampled_texture, texture_sampler, texture_coordinates);

    if (fragment_color.a < 0.1) {
        discard;
    }

    return fragment_color;
}

fn position_data(vertex_index: u32) -> vec4<f32> {
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
