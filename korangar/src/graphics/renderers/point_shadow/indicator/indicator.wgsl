struct Constants {
    light_position: vec4<f32>,
    upper_left: vec4<f32>,
    upper_right: vec4<f32>,
    lower_left: vec4<f32>,
    lower_right: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) texture_coordinates: vec2<f32>,
}

struct FragmentInput {
    @location(0) world_position: vec4<f32>,
    @location(1) texture_coordinates: vec2<f32>,
}

@group(0) @binding(0) var sampled_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(1) @binding(0) var<uniform> view_projection: mat4x4<f32>;

var<push_constant> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    output.world_position = position_data(vertex_index);
    output.position = view_projection * output.world_position;
    output.texture_coordinates = uv_data(vertex_index);
    return output;
}

@fragment
fn fs_main(fragment: FragmentInput) -> @builtin(frag_depth) f32 {
    let fragment_color = textureSample(sampled_texture, texture_sampler, fragment.texture_coordinates);

    if (fragment_color.a < 0.1) {
        discard;
    }

    let light_distance = length(fragment.world_position - constants.light_position);

    return light_distance / 256.0;
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
