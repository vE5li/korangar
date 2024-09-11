struct Constants {
    top_left: vec2<f32>,
    bottom_left: vec2<f32>,
    top_right: vec2<f32>,
    bottom_right: vec2<f32>,
    texture_top_left: vec2<f32>,
    texture_bottom_left: vec2<f32>,
    texture_top_right: vec2<f32>,
    texture_bottom_right: vec2<f32>,
    color0: vec2<f32>,
    color1: vec2<f32>,
}

@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;

var<push_constant> constants: Constants;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = position_data(vertex_index);

    var output: VertexOutput;
    output.position = vec4<f32>(positions.xy, 0.0, 1.0);
    output.texture_coordinates = positions.zw;
    return output;
}

@fragment
fn fs_main(@location(0) texture_coordinates: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, texture_coordinates) * vec4<f32>(constants.color0, constants.color1);
}

fn position_data(vertex_index: u32) -> vec4<f32> {
    switch (vertex_index) {
        case 0u: {
            return vec4<f32>(constants.top_left, constants.texture_top_left);
        }
        case 1u: {
            return vec4<f32>(constants.bottom_left, constants.texture_bottom_left);
        }
        case 2u: {
            return vec4<f32>(constants.top_right, constants.texture_top_right);
        }
        case 3u: {
            return vec4<f32>(constants.top_right, constants.texture_top_right);
        }
        case 4u: {
            return vec4<f32>(constants.bottom_left, constants.texture_bottom_left);
        }
        default: {
            return vec4<f32>(constants.bottom_right, constants.texture_bottom_right);
        }
    }
}
