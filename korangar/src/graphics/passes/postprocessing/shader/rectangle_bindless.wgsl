struct InstanceData {
    color: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    rectangle_type: u32,
    texture_index: i32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) instance_index: u32,
}

@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(1) @binding(1) var textures: binding_array<texture_2d<f32>>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let instance = instance_data[instance_index];

    let vertex = vertex_data(vertex_index);

    let clip_size = instance.screen_size * 2.0;
    let position = screen_to_clip_space(instance.screen_position) + vertex.xy * clip_size;

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.texture_coordinates = instance.texture_position + vertex.zw * instance.texture_size;
    output.instance_index = instance_index;
    return output;
}

/// The range of the SDF border defines the outline of an SDF. msdfgen calls this pxrange.
/// We normaly use pxrange of 8 px when creating 64x64 SDFs, which results in an out border of 4 px in texture space.
const BORDER_WIDTH: f32 = 0.5;
const EDGE_VALUE: f32 = 0.5;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instance_data[input.instance_index];

    var color: vec4<f32> = instance.color;

    switch (instance.rectangle_type) {
        case 1u: {
            // SDF
            let distance = textureSample(textures[instance.texture_index], linear_sampler, input.texture_coordinates).r;
            let aa_width = fwidth(distance);

            color *= vec4(step(EDGE_VALUE, distance));

            // Outside outline
            if (distance > EDGE_VALUE - BORDER_WIDTH && distance < EDGE_VALUE) {
                let bias = 0.1;
                let border_max = (EDGE_VALUE - BORDER_WIDTH) + bias;

                // Transition from transparent to black outline
                let outer_alpha = smoothstep(
                    border_max,
                    border_max + aa_width,
                    distance
                );
                color = vec4<f32>(0.0, 0.0, 0.0, outer_alpha);
            }
            // Inside outline
            else if (distance >= EDGE_VALUE && distance < EDGE_VALUE + aa_width) {
                // Transition from black outline to fill color
                let inner_blend = smoothstep(
                    EDGE_VALUE,
                    EDGE_VALUE + aa_width,
                    distance
                );
                color = mix(vec4<f32>(0.0, 0.0, 0.0, 1.0), instance.color, inner_blend);
            }
        }
        case 2u: {
            // Sprite (linear filtering)
            color *= textureSample(textures[instance.texture_index], linear_sampler, input.texture_coordinates);
        }
        case 3u: {
            // Sprite (nearest filtering)
            color *= textureSample(textures[instance.texture_index], nearest_sampler, input.texture_coordinates);
        }
        default: {}
    }

    return color;
}

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  w
// 0             0  0  0  0
// 1             1  0  1  0
// 2             1 -1  1  1
// 3             1 -1  1  1
// 4             0 -1  0  1
// 5             0  0  0  0
//
// (x,y) are the vertex position
// (z,w) are the UV coordinates
fn vertex_data(vertex_index: u32) -> vec4<f32> {
    let index = 1u << vertex_index;
    let x = f32((index & 0xEu) != 0u);
    let y = f32((index & 0x1Cu) != 0u);
    return vec4<f32>(x, -y, x, y);
}

fn screen_to_clip_space(screen_coords: vec2<f32>) -> vec2<f32> {
    let x = (screen_coords.x * 2.0) - 1.0;
    let y = -(screen_coords.y * 2.0) + 1.0;
    return vec2<f32>(x, y);
}
