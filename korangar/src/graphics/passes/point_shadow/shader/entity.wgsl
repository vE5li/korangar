struct PassUniforms {
    view_projection: mat4x4<f32>,
    light_position: vec4<f32>,
    animation_timer: f32
}

struct InstanceData {
    world: mat4x4<f32>,
    frame_part_transform: mat4x4<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    frame_size: vec2<f32>,
    extra_depth_offset: f32,
    depth_offset: f32,
    curvature: f32,
    mirror: u32,
    texture_index: i32,
    alpha: f32,
}

struct Vertex {
    position: vec3<f32>,
    texture_coordinates: vec2<f32>,
    depth_multiplier: f32,
    curvature_multiplier: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) depth_offset: f32,
    @location(3) curvature: f32,
    @location(4) @interpolate(flat) original_depth_offset: f32,
    @location(5) @interpolate(flat) original_curvature: f32,
    @location(6) alpha: f32,
}

@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var<uniform> pass_uniforms: PassUniforms;
@group(2) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(3) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let instance = instance_data[instance_index];
    let vertex = vertex_data(vertex_index);
    let frame_part_vertex = instance.frame_part_transform * vec4<f32>(vertex.position, 1.0);

    var output: VertexOutput;
    output.world_position = instance.world * frame_part_vertex;
    output.position = pass_uniforms.view_projection * output.world_position;
    output.texture_coordinates = instance.texture_position + vertex.texture_coordinates * instance.texture_size;

    if (instance.mirror != 0u) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    let SPRITE_MAX_SIZE_X = 400.0;
    let SPRITE_MAX_SIZE_Y = 400.0;

    // Values are represented as proportions ranging from -1 to 1
    let proportion_x = instance.frame_size.x / SPRITE_MAX_SIZE_X;
    let proportion_y = instance.frame_size.y / SPRITE_MAX_SIZE_Y;

    // The depth multiplier and curvature multiplier is derived from the truth table of vertex_data
    // Because we have to transform the vertex of the frame part, we can't use the depth and curvature
    // directly and are using the fact, that y / depth and x / curvature correlate to each other.
    // An offset is also added for frame parts not stay at the same depth.
    output.depth_offset = (frame_part_vertex.y - 1.0) * proportion_y + instance.extra_depth_offset;
    output.curvature = frame_part_vertex.x * proportion_x;

    output.original_depth_offset = instance.depth_offset;
    output.original_curvature = instance.curvature;
    output.alpha = instance.alpha;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @builtin(frag_depth) f32 {
    let diffuse_color = textureSample(texture, nearest_sampler, input.texture_coordinates);
    if (diffuse_color.a != 1.0 || input.alpha != 1.0) {
        discard;
    }

    // TODO: This is only a temporary change, we will fix the depth_offset later.
    // The idea for the temporary change is to create two parabolas to correct the plane inclination.
    let sign_depth_offset = select(2.0, -2.0, input.depth_offset < 0.0);
    let scaled_depth_offset = sign_depth_offset * pow(input.depth_offset, 2.0) * input.original_depth_offset;
    let scaled_curvature_offset = (0.5 - pow(input.curvature, 2.0)) * input.original_curvature;

    // Point shadows use a projection matrix, so depth is non-linear.
    let absolute_depth = nonLinearToLinear(input.position.z);
    let adjusted_linear_z: f32 = 3.5 + absolute_depth - scaled_depth_offset - scaled_curvature_offset;

    // TODO: NHA We need to save the distance from this fragment position (with the adjusted depth value) to the light.
    return 0.0;
}

fn nonLinearToLinear(nonlinear_depth: f32) -> f32 {
    const NEAR_PLANE = 0.1;
    return NEAR_PLANE * (nonlinear_depth + 1e-7);
}

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  u  v   d   c
// 0            -1  2  1  0  0   1  -1
// 1            -1  0  1  0  1  -1  -1
// 2             1  2  1  1  0   1   1
// 3             1  2  1  1  0   1   1
// 4            -1  0  1  0  1  -1  -1
// 5             1  0  1  1  1  -1   1
//
// (x,y,z) are the vertex position
// (u,v) are the UV coordinates
// (depth) is the depth multiplier
// (curve) is the curvature multiplier
fn vertex_data(vertex_index: u32) -> Vertex {
    let index = 1u << vertex_index;

    let case0 = i32((index & 0x13u) != 0u);
    let case1 = i32((index & 0x0Du) != 0u);

    let x = f32(1 - 2 * case0);
    let y = f32(2 * case1);
    let z = 1.0;
    let u = f32(1 - case0);
    let v = f32(1 - case1);
    let depth = y - 1.0;
    let curve = x;

    return Vertex(vec3<f32>(x, y, z), vec2<f32>(u, v), depth, curve);
}
