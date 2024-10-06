struct Matrices {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
}

struct Constants {
    world: mat4x4<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    identifier_high: u32,
    identifier_low: u32,
    mirror: u32,
}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) texture_coordinates: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
}

@group(0) @binding(0) var<uniform> matrices: Matrices;
@group(0) @binding(1) var sampled_texture: texture_2d<f32>;
@group(0) @binding(2) var texture_sampler: sampler;

var<push_constant> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertex = vertex_data(vertex_index);

    var output: VertexOutput;
    output.position = matrices.projection * matrices.view * constants.world * vec4<f32>(vertex.position, 1.0);
    output.texture_coordinates = constants.texture_position + vertex.texture_coordinates * constants.texture_size;

    if (constants.mirror != 0u) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    return output;
}

@fragment
fn fs_main(@location(0) texture_coordinates: vec2<f32>) -> @location(0) vec2<u32> {
    let diffuse_color = textureSample(sampled_texture, texture_sampler, texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    return vec2<u32>(constants.identifier_low, constants.identifier_high);
}

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  u  v
// 0            -1  2  0  1  0
// 1            -1  0  0  1  1
// 2             1  2  0  0  0
// 3             1  2  0  0  0
// 4            -1  0  0  1  1
// 5             1  0  0  0  1
//
// (x,y,z) are the vertex position
// (u,v) are the UV coordinates
fn vertex_data(vertex_index: u32) -> Vertex {
    let index = 1u << vertex_index;

    let case0 = i32((index & 0x13u) != 0u);
    let case1 = i32((index & 0x0Du) != 0u);

    let x = f32(1 - 2 * case0);
    let y = f32(2 * case1);
    let z = 0.0;
    let u = f32(case0);
    let v = f32(1 - case1);

    return Vertex(vec3<f32>(x, y, z), vec2<f32>(u, v));
}
