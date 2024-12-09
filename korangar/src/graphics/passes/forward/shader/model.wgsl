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
    shadow_noise_seed: f32,
    shadow_quality: u32,
}

struct DirectionalLightUniforms {
    view_projection: mat4x4<f32>,
    color: vec4<f32>,
    direction: vec4<f32>,
    bound_scale: f32,
}

struct PointLight {
    position: vec4<f32>,
    color: vec4<f32>,
    range: f32,
    texture_index: i32,
}

struct InstanceData {
    world: mat4x4<f32>,
    inv_world: mat4x4<f32>,
}

struct TileLightIndices {
    indices: array<u32, 256>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(3) color: vec3<f32>,
}

const MIP_SCALE: f32 = 0.25;
const ALPHA_CUTOFF: f32 = 0.4;
const TILE_SIZE: u32 = 16;
const TAU: f32 = 6.28318548;
const PCF_RADIUS: f32 = 0.025;
const BLUR_SCALE: f32 = 0.015;
const TOTAL_SAMPLE_COUNT: u32 = 32;
const ESTIMATION_SAMPLE_COUNT: u32 = 8;

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(2) var linear_sampler: sampler;
@group(0) @binding(3) var texture_sampler: sampler;
@group(0) @binding(4) var noise_sampler: sampler;
@group(0) @binding(5) var shadow_map_sampler: sampler_comparison;
@group(1) @binding(0) var<uniform> directional_light: DirectionalLightUniforms;
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var<storage, read> point_lights: array<PointLight>;
@group(1) @binding(3) var light_count_texture: texture_2d<u32>;
@group(1) @binding(4) var<storage, read> tile_light_indices: array<TileLightIndices>;
@group(1) @binding(5) var point_shadow_maps: texture_depth_cube_array;
@group(1) @binding(6) var blue_noise: texture_2d<f32>;
@group(2) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(3) @binding(0) var texture: texture_2d<f32>;

override ALPHA_TO_COVERAGE_ACTIVATED: bool;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
    @location(3) color: vec3<f32>,
    @location(4) wind_affinity: f32,
    @location(5) instance_id: u32
) -> VertexOutput {
    let instance = instance_data[instance_id];

    let world_position = instance.world * vec4<f32>(position, 1.0);
    let wind_position = world_position + vec4<f32>(global_uniforms.animation_timer);
    let offset = vec4<f32>(sin(wind_position.x), 0.0, sin(wind_position.z), 0.0) * wind_affinity;
    let final_world_position = world_position + offset;

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * final_world_position;
    output.world_position = final_world_position;
    output.normal = normalize((instance.inv_world * vec4<f32>(normal, 0.0)).xyz);
    output.texture_coordinates = texture_coordinates;
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var diffuse_color: vec4<f32>;
    var alpha_channel: f32;

    if (ALPHA_TO_COVERAGE_ACTIVATED) {
        diffuse_color = textureSample(texture, texture_sampler, input.texture_coordinates);
        alpha_channel = diffuse_color.a;
    } else {
        diffuse_color = textureSample(texture, texture_sampler, input.texture_coordinates);
        alpha_channel = textureSampleLevel(texture, nearest_sampler, input.texture_coordinates, 0.0).a;
    }

    // Calculate which tile this fragment belongs to
    let pixel_position = vec2<u32>(floor(input.position.xy));
    let tile_x = pixel_position.x / TILE_SIZE;
    let tile_y = pixel_position.y / TILE_SIZE;
    let tile_count_x = (global_uniforms.forward_size.x + TILE_SIZE - 1u) / TILE_SIZE;
    let tile_index = tile_y * tile_count_x + tile_x;

    // Get the number of lights affecting this tile
    let light_count = textureLoad(light_count_texture, vec2<u32>(tile_x, tile_y), 0).r;

    if (ALPHA_TO_COVERAGE_ACTIVATED) {
        // Apply mip level scaling for better mipmap coverage
        let texture_size = vec2<f32>(textureDimensions(texture, 0));
        let coverage = saturate(alpha_channel * (1.0 + max(0.0, calculate_mip_level(input.texture_coordinates * texture_size)) * MIP_SCALE));

        // Apply screen-space derivative scaling for better alpha to coverage anti-aliasing
        alpha_channel = saturate((coverage - ALPHA_CUTOFF) / max(fwidth(coverage), 0.0001) + 0.5);
    } else if (alpha_channel == 0.0) {
        discard;
    }

    let normal = normalize(input.normal);

    // Ambient light
    var ambient_light_contribution = global_uniforms.ambient_color.rgb;

    // Directional light
    let light_direction = normalize(-directional_light.direction.xyz);
    let light_percent = max(dot(light_direction, normal), 0.0);

    // Shadow calculation
    let shadow_position = directional_light.view_projection * input.world_position;
    var shadow_coords = shadow_position.xyz / shadow_position.w;
    let shadow_map_dimensions = textureDimensions(shadow_map);
    let bias = get_oriented_bias(normal, light_direction, shadow_map_dimensions);
    let world_position = input.world_position.xyz / input.world_position.w;
    shadow_coords = vec3<f32>(clip_to_screen_space(shadow_coords.xy), shadow_coords.z + bias);

    var visibility: f32;

    switch (global_uniforms.shadow_quality) {
        case 1u: {
            visibility = get_low_quality_soft_shadows(shadow_coords);
        }
        case 2u: {
            visibility = get_medium_quality_soft_shadows(shadow_coords, shadow_map_dimensions);
        }
        case 3u: {
            visibility = get_high_quality_soft_shadows(world_position, shadow_coords, normal, light_direction);
        }
        default: {
            // Hard shadow
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
        let light_direction = normalize(input.world_position.xyz - light.position.xyz);
        let light_percent = max(dot(light_direction, normal), 0.0);
        let light_distance = length(light.position.xyz - input.world_position.xyz);
        var visibility = 1.0;

        if (light.texture_index != 0) {
            let flipped_light_direction = vec3<f32>(light_direction.x, -light_direction.y, light_direction.z);
            let shadow_map_depth = textureSample(point_shadow_maps, linear_sampler, flipped_light_direction, light.texture_index - 1);
            let bias = clamp(0.05 * tan(acos(light_percent)), 0.0, 0.005);
            let mapped_distance = light_distance / 255.9;
            // TODO: NHA use reverse Z projection also for point shadows
            visibility = f32(mapped_distance - bias < shadow_map_depth);
        }

        let intensity = 10.0;
        let attenuation = calculate_attenuation(light_distance, light.range);
        point_light_contribution += (light.color.rgb * intensity) * light_percent * attenuation * visibility;
    }

    let base_color = diffuse_color.rgb * input.color;
    let light_contributions = saturate(ambient_light_contribution + directional_light_contribution + point_light_contribution);
    var color = base_color.rgb * light_contributions;

    if (global_uniforms.enhanced_lighting == 0) {
        color = color_balance(color, -0.01, 0.0, 0.0);
    }

    return vec4<f32>(color, diffuse_color.a);
}

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

// Quadratic Attenuation with smooth falloff
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

fn calculate_mip_level(texture_coordinate: vec2<f32>) -> f32 {
    let dx = dpdx(texture_coordinate);
    let dy = dpdy(texture_coordinate);
    let delta_max_squared = max(dot(dx, dx), dot(dy, dy));
    return max(0.0, 0.5 * log2(delta_max_squared));
}

// Based on http://the-witness.net/news/2013/09/shadow-mapping-summary-part-1/
fn get_low_quality_soft_shadows(shadow_coords: vec3<f32>) -> f32 {
    // TODO: NHA choose a constant that creates a shadaw that mimics the size of the HQ shadow.
    let scale = 0.0001 * get_filter_size();
    let uv = shadow_coords.xy;
    let base_uv = uv;
    let base_st = fract(uv / scale);

    let s = base_st.x;
    let t = base_st.y;

    var sum = 0.0;

    let uw0 = (5.0 * s - 6.0);
    let uw1 = (11.0 * s - 28.0);
    let uw2 = -(11.0 * s + 17.0);
    let uw3 = -(5.0 * s + 1.0);

    let u0 = ((4.0 * s - 5.0) / uw0 - 3.0) * scale.x;
    let u1 = ((4.0 * s - 16.0) / uw1 - 1.0) * scale.x;
    let u2 = (-(7.0 * s + 5.0) / uw2 + 1.0) * scale.x;
    let u3 = (-s / uw3 + 3.0) * scale.x;

    let vw0 = (5.0 * t - 6.0);
    let vw1 = (11.0 * t - 28.0);
    let vw2 = -(11.0 * t + 17.0);
    let vw3 = -(5.0 * t + 1.0);

    let v0 = ((4.0 * t - 5.0) / vw0 - 3.0) * scale.y;
    let v1 = ((4.0 * t - 16.0) / vw1 - 1.0) * scale.y;
    let v2 = (-(7.0 * t + 5.0) / vw2 + 1.0) * scale.y;
    let v3 = (-t / vw3 + 3.0) * scale.y;

    sum += uw0 * vw0 * sample_shadow_map(base_uv, u0, v0, shadow_coords.z);
    sum += uw1 * vw0 * sample_shadow_map(base_uv, u1, v0, shadow_coords.z);
    sum += uw2 * vw0 * sample_shadow_map(base_uv, u2, v0, shadow_coords.z);
    sum += uw3 * vw0 * sample_shadow_map(base_uv, u3, v0, shadow_coords.z);

    sum += uw0 * vw1 * sample_shadow_map(base_uv, u0, v1, shadow_coords.z);
    sum += uw1 * vw1 * sample_shadow_map(base_uv, u1, v1, shadow_coords.z);
    sum += uw2 * vw1 * sample_shadow_map(base_uv, u2, v1, shadow_coords.z);
    sum += uw3 * vw1 * sample_shadow_map(base_uv, u3, v1, shadow_coords.z);

    sum += uw0 * vw2 * sample_shadow_map(base_uv, u0, v2, shadow_coords.z);
    sum += uw1 * vw2 * sample_shadow_map(base_uv, u1, v2, shadow_coords.z);
    sum += uw2 * vw2 * sample_shadow_map(base_uv, u2, v2, shadow_coords.z);
    sum += uw3 * vw2 * sample_shadow_map(base_uv, u3, v2, shadow_coords.z);

    sum += uw0 * vw3 * sample_shadow_map(base_uv, u0, v3, shadow_coords.z);
    sum += uw1 * vw3 * sample_shadow_map(base_uv, u1, v3, shadow_coords.z);
    sum += uw2 * vw3 * sample_shadow_map(base_uv, u2, v3, shadow_coords.z);
    sum += uw3 * vw3 * sample_shadow_map(base_uv, u3, v3, shadow_coords.z);

    return sum * (1.0 / 2704.0);
}

fn sample_shadow_map(base_uv: vec2<f32>, u: f32, v: f32, depth: f32) -> f32 {
    return textureSampleCompare(
        shadow_map,
        shadow_map_sampler,
        base_uv + vec2<f32>(u, v),
        depth
    );
}

fn get_medium_quality_soft_shadows(shadow_coords: vec3<f32>, shadow_map_dimensions: vec2<u32>) -> f32 {

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

    for (var y: i32 = -gaussian_offset; y <= gaussian_offset; y += 2) {
        for (var x: i32 = -gaussian_offset; x <= gaussian_offset; x += 2) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;

            // Calculate Gaussian weight based on distance from center
            let distance_squared = f32(x * x + y * y) / f32(gaussian_offset * gaussian_offset);
            let weight = exp(-distance_squared * 2.0);

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

fn get_high_quality_soft_shadows(world_position: vec3<f32>, shadow_coords: vec3<f32>, normal: vec3<f32>, light_direction: vec3<f32>) -> f32 {
    let filter_size = get_filter_size();

    var shadow = 0.0;

    // Take the estimation samples at the edge.
    for (var index = 0u; index < ESTIMATION_SAMPLE_COUNT; index++) {
        let offset = get_jittered_offset(world_position, index, true) * filter_size;
        shadow += textureSampleCompare(
            shadow_map,
            shadow_map_sampler,
            shadow_coords.xy + offset,
            shadow_coords.z
        );
    }
    shadow *= (1.0/f32(ESTIMATION_SAMPLE_COUNT));

    let NdotL = max(dot(normal, light_direction), 0.0);

    // Check if we're in penumbra region.
    if ((shadow - 1.0) * shadow * NdotL != 0.0) {
        // Adjust weight for additional samples.
        shadow *= f32(ESTIMATION_SAMPLE_COUNT) / f32(TOTAL_SAMPLE_COUNT);

        // Take the rest of the samples.
        for (var index = ESTIMATION_SAMPLE_COUNT; index < TOTAL_SAMPLE_COUNT; index++) {
            let offset = get_jittered_offset(world_position, index, false) * filter_size;
            shadow += textureSampleCompare(
                shadow_map,
                shadow_map_sampler,
                shadow_coords.xy + offset,
                shadow_coords.z
            ) * (1.0 / f32(TOTAL_SAMPLE_COUNT));
        }
    }

    return shadow;
}


fn get_jittered_offset(world_position: vec3<f32>, sample_index: u32, is_estimation: bool) -> vec2<f32> {
    let sample_count = select(f32(TOTAL_SAMPLE_COUNT), f32(ESTIMATION_SAMPLE_COUNT), is_estimation);
    let angle = (f32(sample_index) / sample_count) * TAU;

    let random = global_uniforms.shadow_noise_seed;
    let noise_coordinate = (world_position.xz + vec2<f32>(f32(sample_index) * random)) ;
    let noise = textureSample(blue_noise, noise_sampler, noise_coordinate).xy;

    var radius: f32;

    if (is_estimation) {
        // Estimation samples near the edge (0.8-1.0 range).
        radius = (0.8 + noise.x * 0.2) * PCF_RADIUS;
    } else {
        // Regular samples distributed across the disk.
        radius = sqrt(noise.x) * PCF_RADIUS;
    }

    // Calculate final offset
    let offset = vec2<f32>(
        cos(angle) * radius,
        sin(angle) * radius
    ) * BLUR_SCALE;

    return offset;
}

fn get_filter_size() -> vec2<f32> {
    // Scale our sampling based on how much we've zoomed.
    return vec2<f32>(1.0) * directional_light.bound_scale;
}

// Based on "Shadow Techniques from Final Fantasy XVI" by Sammy Fatnassi (2023)
fn get_oriented_bias(normal: vec3<f32>, light_direction: vec3<f32>, shadow_map_dimensions: vec2<u32>) -> f32 {
    let k_oriented_bias = 0.001;
    let is_facing_light = dot(normal, light_direction) > 0.0;
    // sic! We use reverse Z projection!
    return select(-k_oriented_bias, k_oriented_bias, is_facing_light);
}
