struct Constants {
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    screen_clip: vec4<f32>,
    color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
}

@group(0) @binding(0) var sprite_texture: texture_2d<f32>;
@group(0) @binding(1) var sprite_sampler: sampler;

var<push_constant> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertex = vertex_data(vertex_index);
    let clip_size = constants.screen_size * 2.0;
    let position = screen_to_clip_space(constants.screen_position) + vertex.xy * clip_size;

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.texture_coordinates = vertex.zw;
    return output;
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>, @location(0) texture_coordinates: vec2<f32>) -> @location(0) vec4<f32> {
    if (position.x < constants.screen_clip.x || position.y < constants.screen_clip.y ||
        position.x > constants.screen_clip.z || position.y > constants.screen_clip.w) {
        discard;
    }

    return textureSample(sprite_texture, sprite_sampler, texture_coordinates) * constants.color;
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
