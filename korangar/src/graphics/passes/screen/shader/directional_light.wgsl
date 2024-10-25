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

struct LightUniforms {
    view_projection: mat4x4<f32>,
    color: vec4<f32>,
    direction: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var diffuse_buffer: texture_multisampled_2d<f32>;
@group(1) @binding(1) var normal_buffer: texture_multisampled_2d<f32>;
@group(1) @binding(3) var depth_buffer: texture_depth_multisampled_2d;
@group(1) @binding(4) var shadow_map: texture_depth_2d;
@group(2) @binding(0) var<uniform> light_uniforms: LightUniforms;

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

    let depth: f32 = textureLoad(depth_buffer, pixel_coord, sample_index);
    var pixel_position_world_space: vec4<f32> = global_uniforms.inverse_view_projection * vec4<f32>(fragment_position, depth, 1.0);
    pixel_position_world_space /= pixel_position_world_space.w;

    var normal: vec3<f32> = normalize(textureLoad(normal_buffer, pixel_coord, sample_index).rgb);
    var light_percent: f32 = dot(normalize(-light_uniforms.direction.xyz), normal);
    light_percent = clamp(light_percent, 0.0, 1.0);

    // Triangles flicker black if the bias is too low.
    let bias: f32 = clamp(0.0025 * tan(acos(light_percent)), 0.0, 0.0005);

    let light_position: vec4<f32> = light_uniforms.view_projection * pixel_position_world_space;
    var light_coords: vec3<f32> = light_position.xyz / light_position.w;

    let uv: vec2<f32> = clip_to_screen_space(light_coords.xy);
    let shadow_map_depth: f32 = textureSample(shadow_map, linear_sampler, uv);
    let visibility: f32 = select(0.0, 1.0, light_coords.z - bias < shadow_map_depth);

    let diffuse: vec3<f32> = textureLoad(diffuse_buffer, pixel_coord, sample_index).rgb;
    return light_percent * light_uniforms.color.rgb * diffuse * visibility;
}

fn clip_to_screen_space(ndc: vec2<f32>) -> vec2<f32> {
    let u = (ndc.x + 1.0) / 2.0;
    let v = (1.0 - ndc.y) / 2.0;
    return vec2<f32>(u, v);
}
