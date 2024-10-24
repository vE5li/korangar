struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
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
    depth_offset: f32,
    curvature: f32,
    mirror: u32,
    texture_index: i32,
}

struct Vertex {
    position: vec3<f32>,
    texture_coordinates: vec2<f32>,
    depth_multiplier: f32,
    curvature_multiplier: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) depth_offset: f32,
    @location(3) curvature: f32,
    @location(4) @interpolate(flat) original_depth_offset: f32,
    @location(5) @interpolate(flat) original_curvature: f32,
}

struct FragmentOutput {
    @location(0) fragment_color: vec4<f32>,
    @location(1) fragment_normal: vec4<f32>,
    @builtin(frag_depth) frag_depth: f32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(2) @binding(0) var texture: texture_2d<f32>;

override near_plane: f32;

// Small value to prevent division by zero.
const epsilon: f32 = 1e-7;

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

    if (instance.mirror != 0u) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    let rotated = rotateY(vec3<f32>(global_uniforms.view[2].x, 0, global_uniforms.view[2].z), vertex.position.x);
    output.normal = vec3<f32>(-rotated.x, rotated.y, rotated.z);
    output.depth_offset = vertex.depth_multiplier;
    output.curvature = vertex.curvature_multiplier;
    output.original_depth_offset = instance.depth_offset;
    output.original_curvature = instance.curvature;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    let diffuse_color = textureSample(texture, linear_sampler, input.texture_coordinates);
    if (diffuse_color.a != 1.0) {
        discard;
    }

    let scaled_depth_offset = pow(input.depth_offset, 2.0) * input.original_depth_offset;
    let scaled_curvature_offset = (0.5 - pow(input.curvature, 2.0)) * input.original_curvature;

    let linear_z: f32 = nonLinearToLinear(input.position.z);
    // We add the offsets in linear view space.
    let adjusted_linear_z: f32 = -2.0 + linear_z - scaled_depth_offset - scaled_curvature_offset;
    let non_linear_z: f32 = linearToNonLinear(adjusted_linear_z);
    let clamped_depth = clamp(non_linear_z, 0.0, 1.0);

    var output: FragmentOutput;
    output.fragment_color = diffuse_color;
    output.fragment_normal = vec4<f32>(normalize(input.normal), 1.0);
    output.frag_depth = clamped_depth;
    return output;
}

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  u  v  d  c
// 0            -1  2  1  0  0  1 -1
// 1            -1  0  1  0  1  0 -1
// 2             1  2  1  1  0  1  1
// 3             1  2  1  1  0  1  1
// 4            -1  0  1  0  1  0 -1
// 5             1  0  1  1  1  0  1
//
// (x,y,z) are the vertex position
// (u,v) are the UV coordinates
// (depth) is the depth multiplier
// (curve) is the cuvature multiplier
fn vertex_data(vertex_index: u32) -> Vertex {
    let index = 1u << vertex_index;

    let case0 = i32((index & 0x13u) != 0u);
    let case1 = i32((index & 0x0Du) != 0u);

    let x = f32(1 - 2 * case0);
    let y = f32(2 * case1);
    let z = 1.0;
    let u = f32(1 - case0);
    let v = f32(1 - case1);
    let depth = f32(case1);
    let curve = u * 2.0 - 1.0;

    return Vertex(vec3<f32>(x, y, z), vec2<f32>(u, v), depth, curve);
}


fn rotateY(direction: vec3<f32>, angle: f32) -> vec3<f32> {
    let s = sin(angle);
    let c = cos(angle);
    let rotation_matrix = mat3x3<f32>(
        c, 0.0, -s,
        0.0, 1.0, 0.0,
        s, 0.0, c
    );
    return rotation_matrix * direction;
}

fn linearToNonLinear(linear_depth: f32) -> f32 {
    return near_plane / (linear_depth + epsilon);
}

fn nonLinearToLinear(non_linear_depth: f32) -> f32 {
    return near_plane / (non_linear_depth + epsilon);
}
