struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    view:mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    pointer_position: vec2<u32>,
    animation_timer: f32,
    day_timer: f32,
    water_level: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(1) @binding(2) var water_buffer: texture_multisampled_2d<f32>;
@group(1) @binding(3) var depth_buffer: texture_depth_multisampled_2d;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    let vertex = vec2<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0));

    var output: VertexOutput;
    output.position = vec4<f32>(vertex, 0.0, 1.0);
    output.fragment_position = vertex;
    return output;
}

@fragment
fn fs_main(
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
) -> @location(0) vec4<f32> {
    var blended = vec3<f32>(0.0);
    for (var sample_id: i32 = 0; sample_id < 4; sample_id++) {
        blended += calculate_sample(position, fragment_position, sample_id);
    }
    return vec4<f32>(blended / 4.0, 1.0);
}

fn calculate_sample(position: vec4<f32>, fragment_position: vec2<f32>, sample_index: i32) -> vec3<f32> {
    let pixel_coord = vec2<i32>(position.xy);

    let water = textureLoad(water_buffer, pixel_coord, sample_index).r;
    let depth = textureLoad(depth_buffer, pixel_coord, sample_index);

    var pixel_position_world_space = global_uniforms.inverse_view_projection * vec4<f32>(fragment_position, depth, 1.0);
    pixel_position_world_space /= pixel_position_world_space.w;

    let delta = global_uniforms.water_level - pixel_position_world_space.y;
    let factor = max(0.2, delta / 30.0);

    return vec3<f32>(factor, min(factor / 2.0, 0.3), min(factor / 10.0, 0.1)) * water;
}
