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
    position: vec4<f32>,
    color: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    range: f32,
    texture_index: i32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
    @location(1) instance_index: u32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var diffuse_buffer: texture_multisampled_2d<f32>;
@group(1) @binding(1) var normal_buffer: texture_multisampled_2d<f32>;
@group(1) @binding(3) var depth_buffer: texture_depth_multisampled_2d;
@group(1) @binding(5) var point_shadow_maps: texture_depth_cube_array;
@group(2) @binding(0) var<storage, read> instance_data: array<InstanceData>;

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
    output.fragment_position = vec2<f32>(position);
    output.instance_index = instance_index;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var blended = vec3<f32>(0.0);

    for (var sample_id: i32 = 0; sample_id < 4; sample_id++) {
        blended += calculate_sample(input.position, input.fragment_position, input.instance_index, sample_id);
    }

    return vec4<f32>(blended / 4.0, 1.0);
}

fn calculate_sample(position: vec4<f32>, fragment_position: vec2<f32>, instance_index: u32, sample_index: i32) -> vec3<f32> {
    let instance = instance_data[instance_index];

    let pixel_coord = vec2<i32>(position.xy);

    var diffuse = textureLoad(diffuse_buffer, pixel_coord, sample_index).rgb;
    let normal =textureLoad(normal_buffer, pixel_coord, sample_index).rgb;
    let depth = textureLoad(depth_buffer, pixel_coord, sample_index);

    var pixel_position_world_space = global_uniforms.inverse_view_projection * vec4<f32>(fragment_position, depth, 1.0);
    pixel_position_world_space /= pixel_position_world_space.w;

    let light_direction = normalize(pixel_position_world_space.xyz - instance.position.xyz);
    var light_percent = max(dot(light_direction,  normalize(normal)), 0.0);
    let light_distance = length(instance.position.xyz - pixel_position_world_space.xyz);
    var visibility = 1.0;

    // We use the texture_index to identify shadow caster.
    // For this we save the texture_index as (texture_index += 1);
    if (instance.texture_index != 0) {
        let flipped_light_direction = vec3<f32>(light_direction.x, -light_direction.y, light_direction.z);
        let shadow_map_depth = textureSample(point_shadow_maps, linear_sampler, flipped_light_direction, instance.texture_index - 1);

        var bias = 0.05 * tan(acos(light_percent));
        bias = clamp(bias, 0.0, 0.005);

        let mapped_distance = light_distance / 255.9;
        visibility = f32(mapped_distance - bias < shadow_map_depth);
    }

    light_percent *= min(instance.range / exp(light_distance / 10.0), 0.7) * visibility;
    return light_percent * instance.color.rgb * diffuse.rgb;
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
