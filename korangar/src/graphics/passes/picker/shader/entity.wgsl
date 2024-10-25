struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    screen_size: vec2<u32>,
    pointer_position: vec2<u32>,
    animation_timer: f32,
    day_timer: f32,
    water_level: f32,
    point_light_count: u32,
}

struct InstanceData {
    world: mat4x4<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    texture_index: i32,
    mirror: u32,
    identifier_high: u32,
    identifier_low: u32,
}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) texture_coordinates: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) identifier_high: u32,
    @location(2) identifier_low: u32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(2) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let instance = instance_data[instance_index];
    let vertex = vertex_data(vertex_index);

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * instance.world * vec4<f32>(vertex.position, 1.0);
    output.texture_coordinates = instance.texture_position + vertex.texture_coordinates * instance.texture_size;
    output.identifier_high = instance.identifier_high;
    output.identifier_low = instance.identifier_low;

    if (instance.mirror != 0u) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec2<u32> {
    let diffuse_color = textureSample(texture, nearest_sampler, input.texture_coordinates);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    return vec2<u32>(input.identifier_low, input.identifier_high);
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
