struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    camera_position: vec4<f32>,
    forward_size: vec2<u32>,
    interface_size: vec2<u32>,
    pointer_position: vec2<u32>,
    animation_timer: f32,
    day_timer: f32,
    point_light_count: u32,
    enhanced_lighting: u32,
    shadow_quality: u32,
}

struct DirectionalLightUniforms {
    view_projection: mat4x4<f32>,
    color: vec4<f32>,
    direction: vec4<f32>,
}

struct PointLight {
    position: vec4<f32>,
    color: vec4<f32>,
    range: f32,
    texture_index: i32,
}

struct InstanceData {
    world: mat4x4<f32>,
    frame_part_transform: mat4x4<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    color: vec4<f32>,
    frame_size: vec2<f32>,
    extra_depth_offset: f32,
    depth_offset: f32,
    curvature: f32,
    mirror: u32,
    texture_index: i32,
}

struct TileLightIndices {
    indices: array<u32, 256>,
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
    @location(2) normal: vec3<f32>,
    @location(3) depth_offset: f32,
    @location(4) curvature: f32,
    @location(5) @interpolate(flat) original_depth_offset: f32,
    @location(6) @interpolate(flat) original_curvature: f32,
    @location(7) color: vec4<f32>,
}

struct FragmentOutput {
    @location(0) fragment_color: vec4<f32>,
    @builtin(frag_depth) frag_depth: f32,
}

const TILE_SIZE: u32 = 16;

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(2) var linear_sampler: sampler;
@group(0) @binding(3) var texture_sampler: sampler;
@group(0) @binding(4) var shadow_map_sampler: sampler_comparison;
@group(1) @binding(0) var<uniform> directional_light: DirectionalLightUniforms;
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var<storage, read> point_lights: array<PointLight>;
@group(1) @binding(3) var light_count_texture: texture_2d<u32>;
@group(1) @binding(4) var<storage, read> tile_light_indices: array<TileLightIndices>;
@group(1) @binding(5) var point_shadow_maps: texture_depth_cube_array;
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
    let world_position = instance.world * frame_part_vertex;

    var output: VertexOutput;
    output.world_position = world_position;
    output.position = global_uniforms.view_projection * world_position;
    output.texture_coordinates = instance.texture_position + vertex.texture_coordinates * instance.texture_size;

    if (instance.mirror != 0u) {
        output.texture_coordinates.x = 1.0 - output.texture_coordinates.x;
    }

    let rotated = rotateY(vec3<f32>(global_uniforms.view[2].x, 0, global_uniforms.view[2].z), vertex.position.x);
    output.normal = vec3<f32>(-rotated.x, rotated.y, rotated.z);

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
    output.color = instance.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> FragmentOutput {
    let diffuse_color = textureSample(texture, texture_sampler, input.texture_coordinates);
    var alpha_channel = textureSample(texture, nearest_sampler, input.texture_coordinates).a;
    alpha_channel *= input.color.a;

    if (alpha_channel == 0.0) {
        discard;
    }

    // Calculate which tile this fragment belongs to
    let pixel_position = vec2<u32>(floor(input.position.xy));
    let tile_x = pixel_position.x / TILE_SIZE;
    let tile_y = pixel_position.y / TILE_SIZE;
    let tile_count_x = (global_uniforms.forward_size.x + TILE_SIZE - 1u) / TILE_SIZE;
    let tile_index = tile_y * tile_count_x + tile_x;

    // Get the number of lights affecting this tile
    let light_count = textureLoad(light_count_texture, vec2<u32>(tile_x, tile_y), 0).r;

    let normal = normalize(input.normal);

    // TODO: This is only a temporary change, we will fix the depth_offset later.
    // The idea for the temporary change is to create two parabolas to correct the plane inclination.
    let sign_depth_offset = select(2.0, -2.0, input.depth_offset < 0.0);
    let scaled_depth_offset = sign_depth_offset * pow(input.depth_offset, 2.0) * input.original_depth_offset;
    let scaled_curvature_offset = (0.5 - pow(input.curvature, 2.0)) * input.original_curvature;

    // Adjust world position in view space
    let view_position = global_uniforms.view * input.world_position;
    // Tha magic +2.0 is added, so that entities don't clip into the ground.
    let adjusted_view_position = view_position - vec4<f32>(0.0, 0.0, scaled_depth_offset + scaled_curvature_offset + 2.0, 0.0);
    let adjusted_world_position = global_uniforms.inverse_view * adjusted_view_position;

    // Depth adjustment calculation
    let clip_position = global_uniforms.view_projection * adjusted_world_position;
    let clamped_depth = clamp(clip_position.z / clip_position.w, 0.0, 1.0);

    // Ambient light
    var ambient_light_contribution = global_uniforms.ambient_color.rgb;

    // Directional light
    let light_direction = normalize(-directional_light.direction.xyz);
    var light_percent = 1.0;

    if (global_uniforms.enhanced_lighting != 0) {
        light_percent = max(dot(light_direction, normal), 0.0);
    }

    // Shadow calculation
    let shadow_position = directional_light.view_projection * input.world_position;
    var shadow_coords = shadow_position.xyz / shadow_position.w;
    let bias = 0.002;
    let world_position = input.world_position.xyz / input.world_position.w;
    shadow_coords = vec3<f32>(clip_to_screen_space(shadow_coords.xy), shadow_coords.z + bias);

    var visibility: f32;

    switch (global_uniforms.shadow_quality) {
        case 1u: {
            let shadow_map_dimensions = textureDimensions(shadow_map);
            visibility = get_soft_shadow(shadow_coords, shadow_map_dimensions);
        }
        default: {
            visibility = textureSampleCompare(
                      shadow_map,
                      shadow_map_sampler,
                      shadow_coords.xy,
                      shadow_coords.z
            );
        }
    }

    let directional_light_contribution = directional_light.color.rgb * light_percent * visibility;

    // Point lights
    var point_light_contribution = vec3<f32>(0.0);
    for (var index = 0u; index < light_count; index++) {
        let light_index = tile_light_indices[tile_index].indices[index];
        let light = point_lights[light_index];
        let light_direction = normalize(adjusted_world_position.xyz - light.position.xyz);
        let light_percent = max(dot(light_direction, input.normal), 0.0);
        let light_distance = length(light.position.xyz - adjusted_world_position.xyz);
        var visibility = 1.0;

        if (light.texture_index != 0) {
            let bias = 1.2;
            let distance_to_light = linearToNonLinear(light_distance - bias);

            let closest_distance = textureSample(
                point_shadow_maps,
                linear_sampler,
                light_direction,
                light.texture_index - 1
            );

            visibility = f32(distance_to_light > closest_distance);
        }

        let intensity = 10.0;
        let attenuation = calculate_attenuation(light_distance, light.range);
        point_light_contribution += (light.color.rgb * intensity) * light_percent * attenuation * visibility;
    }

    let base_color = diffuse_color * input.color;
    let light_contributions = saturate(ambient_light_contribution + directional_light_contribution + point_light_contribution);
    var color = base_color.rgb * light_contributions;

    if (global_uniforms.enhanced_lighting == 0) {
        color = color_balance(color, -0.01, 0.0, 0.0);
    }

    var output: FragmentOutput;
    output.fragment_color = vec4<f32>(color, alpha_channel);
    output.frag_depth = clamped_depth;
    return output;
}

// Assuming inputs are in range [-1, 1] where:
// -1 = full shift towards first color (Cyan/Magenta/Yellow)
// +1 = full shift towards second color (Red/Green/Blue)
fn color_balance(color: vec3<f32>, cyan_red: f32, magenta_green: f32, yellow_blue: f32) -> vec3<f32> {
    let rgb = color;

    let adjusted = vec3<f32>(
        rgb.r + cyan_red,
        rgb.g + magenta_green,
        rgb.b + yellow_blue
    );

    return clamp(adjusted, vec3<f32>(0.0), vec3<f32>(1.0));
}

// Quadratic attenuation with smooth falloff
fn calculate_attenuation(distance: f32, range: f32) -> f32 {
    let effective_distance = min(distance, range);
    let normalized_distance = effective_distance / range;
    let attenuation = saturate(1.0 - normalized_distance * normalized_distance);
    return attenuation * attenuation;
}

fn clip_to_screen_space(ndc: vec2<f32>) -> vec2<f32> {
    let u = (ndc.x + 1.0) / 2.0;
    let v = (1.0 - ndc.y) / 2.0;
    return vec2<f32>(u, v);
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

fn get_soft_shadow(shadow_coords: vec3<f32>, shadow_map_dimensions: vec2<u32>) -> f32 {
    var gaussian_offset: i32;
    switch (shadow_map_dimensions.x) {
        case 8192u: {
            gaussian_offset = 8;
        }
        case 4096u: {
            gaussian_offset = 4;
        }
        default: {
            gaussian_offset = 2;
        }
    }

    let texel_size = vec2<f32>(1.0) / vec2<f32>(shadow_map_dimensions);
    let depth = shadow_coords.z;
    var shadow: f32 = 0.0;
    var total_weight: f32 = 0.0;

    let gaussian_offset_pow2 = f32(gaussian_offset * gaussian_offset);
    let sigma_squared = gaussian_offset_pow2 * 0.25;
    let weight_factor = 1.0 / (2.0 * sigma_squared);

    for (var y: i32 = -gaussian_offset; y <= gaussian_offset; y += 2) {
        for (var x: i32 = -gaussian_offset; x <= gaussian_offset; x += 2) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;

            // Calculate Gaussian weight based on distance from center.
            let distance_squared = f32(x * x + y * y);
            let weight = exp(-distance_squared * weight_factor);

            let samples = textureGatherCompare(
                shadow_map,
                shadow_map_sampler,
                shadow_coords.xy + offset,
                depth
            );

            shadow += (samples.x + samples.y + samples.z + samples.w) * weight;
            total_weight += 4.0 * weight;
        }
    }

    return shadow / total_weight;
}

fn linearToNonLinear(linear_depth: f32) -> f32 {
    const NEAR_PLANE = 0.1;
    return NEAR_PLANE / (linear_depth + 1e-7);
}
