struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
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
    frame_part_transform: mat4x4<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    texture_index: i32,
    angle: f32,
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
    @location(3) angle: f32,
}

struct FragmentOutput {
    @location(0) identifier: vec2<u32>,
    @builtin(frag_depth) frag_depth: f32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(2) @binding(0) var texture: texture_2d<f32>;

// Small value to prevent division by zero.
const EPSILON: f32 = 1e-7;
const NEAR_PLANE: f32 = 1.0;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let instance = instance_data[instance_index];
    let vertex = vertex_data(vertex_index);
    let frame_part_vertex = instance.frame_part_transform * vec4<f32>(vertex.position, 1.0);

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * instance.world * frame_part_vertex;
    output.texture_coordinates = instance.texture_position + vertex.texture_coordinates * instance.texture_size;

    if (instance.mirror != 0u) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    output.identifier_high = instance.identifier_high;
    output.identifier_low = instance.identifier_low;
    output.angle = instance.angle;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) ->FragmentOutput {
    // Apply the rotation from action
    let sin_factor = sin(input.angle);
    let cos_factor = cos(input.angle);
    let rotate = vec2(input.texture_coordinates.x - 0.5, input.texture_coordinates.y - 0.5) * mat2x2(cos_factor, sin_factor, -sin_factor, cos_factor);
    let new_input = vec2(clamp(rotate.x + 0.5, 0.0, 1.0), clamp(rotate.y + 0.5, 0.0, 1.0));
    let diffuse_color = textureSample(texture, nearest_sampler, new_input);

    if (diffuse_color.a != 1.0) {
        discard;
    }

    // We add a small offset in linear space, so that entities don't clip into the ground.
    let linear_z: f32 = nonLinearToLinear(input.position.z);
    let adjusted_linear_z: f32 = linear_z - 2.0;
    let non_linear_z: f32 = linearToNonLinear(adjusted_linear_z);
    let clamped_depth = clamp(non_linear_z, 0.0, 1.0);

    var output: FragmentOutput;
    output.identifier = vec2<u32>(input.identifier_low, input.identifier_high);
    output.frag_depth = clamped_depth;
    return output;
}

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  u  v
// 0            -1  2  0  0  0
// 1            -1  0  0  0  1
// 2             1  2  0  1  0
// 3             1  2  0  1  0
// 4            -1  0  0  0  1
// 5             1  0  0  1  1
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
    let u = f32(1 - case0);
    let v = f32(1 - case1);

    return Vertex(vec3<f32>(x, y, z), vec2<f32>(u, v));
}

fn linearToNonLinear(linear_depth: f32) -> f32 {
    return NEAR_PLANE / (linear_depth + EPSILON);
}

fn nonLinearToLinear(non_linear_depth: f32) -> f32 {
    return NEAR_PLANE / (non_linear_depth + EPSILON);
}
