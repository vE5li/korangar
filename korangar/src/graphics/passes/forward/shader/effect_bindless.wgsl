struct InstanceData {
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
    texture_index: i32,
    padding: u32,
}

@group(0) @binding(2) var linear_sampler: sampler;
@group(2) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(2) @binding(1) var textures: binding_array<texture_2d<f32>>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) texture_index: i32,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let instance = instance_data[instance_index];

    let positions = position_data(instance, vertex_index);

    var output: VertexOutput;
    output.position = vec4<f32>(positions.xy, 0.0, 1.0);
    output.texture_coordinates = positions.zw;
    output.color = vec4<f32>(instance.color0, instance.color1);
    output.texture_index = instance.texture_index;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(textures[input.texture_index], linear_sampler, input.texture_coordinates) * input.color;
}

fn position_data(instance: InstanceData, vertex_index: u32) -> vec4<f32> {
    switch (vertex_index) {
        case 0u: {
            return vec4<f32>(instance.top_left, instance.texture_top_left);
        }
        case 1u: {
            return vec4<f32>(instance.bottom_left, instance.texture_bottom_left);
        }
        case 2u: {
            return vec4<f32>(instance.top_right, instance.texture_top_right);
        }
        case 3u: {
            return vec4<f32>(instance.top_right, instance.texture_top_right);
        }
        case 4u: {
            return vec4<f32>(instance.bottom_left, instance.texture_bottom_left);
        }
        default: {
            return vec4<f32>(instance.bottom_right, instance.texture_bottom_right);
        }
    }
}
