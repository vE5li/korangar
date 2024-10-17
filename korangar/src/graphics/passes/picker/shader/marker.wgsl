struct InstanceData {
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    identifier_high: u32,
    identifier_low: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) identifier_high: u32,
    @location(1) identifier_low: u32,
}

@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;

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
    output.position = vec4<f32>(position, 1.0, 1.0);
    output.identifier_high = instance.identifier_high;
    output.identifier_low = instance.identifier_low;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec2<u32> {
    return vec2<u32>(input.identifier_low, input.identifier_high);
}

// Optimized version of the following truth table:
//
// vertex_index  x  y
// 0             0  0
// 1             1  0
// 2             1 -1
// 3             1 -1
// 4             0 -1
// 5             0  0
//
// (x,y) are the vertex position
fn vertex_data(vertex_index: u32) -> vec2<f32> {
    let index = 1u << vertex_index;
    let x = f32((index & 0xEu) != 0u);
    let y = f32((index & 0x1Cu) != 0u);
    return vec2<f32>(x, -y);
}

fn screen_to_clip_space(screen_coords: vec2<f32>) -> vec2<f32> {
    let x = (screen_coords.x * 2.0) - 1.0;
    let y = -(screen_coords.y * 2.0) + 1.0;
    return vec2<f32>(x, y);
}
