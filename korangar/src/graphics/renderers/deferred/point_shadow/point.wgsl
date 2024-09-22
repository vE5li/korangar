struct Matrices {
    screen_to_world: mat4x4<f32>,
}

struct Constants {
    position: vec4<f32>,
    color: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    range: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
}

@group(0) @binding(0) var diffuse_buffer: texture_multisampled_2d<f32>;
@group(0) @binding(1) var normal_buffer: texture_multisampled_2d<f32>;
@group(0) @binding(2) var depth_buffer: texture_depth_multisampled_2d;
@group(0) @binding(3) var<uniform> matrices: Matrices;
@group(0) @binding(4) var shadow_sampler: sampler;
@group(1) @binding(0) var shadow_map: texture_depth_cube;

var<push_constant> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertex = vertex_data(vertex_index);
    let clip_size = constants.screen_size * 2.0;
    let position = screen_to_clip_space(constants.screen_position) + vertex.xy * clip_size;

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.fragment_position = vec2<f32>(position);
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

    let depth = textureLoad(depth_buffer, pixel_coord, sample_index);

    var pixel_position_world_space = matrices.screen_to_world * vec4<f32>(fragment_position, depth, 1.0);
    pixel_position_world_space /= pixel_position_world_space.w;

    let normal = normalize(textureLoad(normal_buffer, pixel_coord, sample_index).rgb);
    let light_direction = normalize(pixel_position_world_space.xyz - constants.position.xyz);

    var light_percent = max(dot(light_direction, normal), 0.0);
    let light_distance = length(constants.position.xyz - pixel_position_world_space.xyz);

    let flipped_light_direction = vec3<f32>(light_direction.x, -light_direction.y, light_direction.z);
    let shadow_map_depth = textureSample(shadow_map, shadow_sampler, flipped_light_direction);

    var bias = 0.05 * tan(acos(light_percent));
    bias = clamp(bias, 0.0, 0.005);

    let mapped_distance = light_distance / 255.9;
    let visibility = mapped_distance - bias < shadow_map_depth;

    light_percent *= min(constants.range / exp(light_distance / 10.0), 0.7) * f32(visibility);

    let diffuse = textureLoad(diffuse_buffer, pixel_coord, sample_index).rgb;
    return light_percent * constants.color.rgb * diffuse;
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
