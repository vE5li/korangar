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

struct TileLightIndices {
    indices: array<u32, 256>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture_coordinates: vec2<f32>,
}

const TILE_SIZE: u32 = 16;

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(2) var linear_sampler: sampler;
@group(0) @binding(4) var shadow_map_sampler: sampler_comparison;
@group(1) @binding(0) var<uniform> directional_light: DirectionalLightUniforms;
@group(1) @binding(1) var shadow_map: texture_depth_2d;
@group(1) @binding(2) var<storage, read> point_lights: array<PointLight>;
@group(1) @binding(3) var light_count_texture: texture_2d<u32>;
@group(1) @binding(4) var<storage, read> tile_light_indices: array<TileLightIndices>;
@group(1) @binding(5) var point_shadow_maps: texture_depth_cube_array;
@group(2) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let world_position = position_data(vertex_index);

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * world_position;
    output.world_position = world_position;
    output.normal = normal_data(vertex_index);
    output.texture_coordinates = uv_data(vertex_index);
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let diffuse_color = textureSample(texture, nearest_sampler, input.texture_coordinates);

    // Calculate which tile this fragment belongs to
    let pixel_position = vec2<u32>(floor(input.position.xy));
    let tile_x = pixel_position.x / TILE_SIZE;
    let tile_y = pixel_position.y / TILE_SIZE;
    let tile_count_x = (global_uniforms.forward_size.x + TILE_SIZE - 1u) / TILE_SIZE;
    let tile_index = tile_y * tile_count_x + tile_x;

    // Get the number of lights affecting this tile
    let light_count = textureLoad(light_count_texture, vec2<u32>(tile_x, tile_y), 0).r;

    if (diffuse_color.a < 0.1) {
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
    let bias = get_oriented_bias(normal, light_direction);
    let world_position = input.world_position.xyz / input.world_position.w;
    shadow_coords = vec3<f32>(clip_to_screen_space(shadow_coords.xy), shadow_coords.z + bias);

    var visibility: f32;

    switch (global_uniforms.shadow_quality) {
        case 0u: {
            visibility = textureSampleCompare(
                      shadow_map,
                      shadow_map_sampler,
                      shadow_coords.xy,
                      shadow_coords.z
            );
        }
        case 1u: {
            let shadow_map_dimensions = textureDimensions(shadow_map);
            visibility = get_pcf_shadow(shadow_coords, shadow_map_dimensions);
        }
        default: {
            visibility = get_pcf_pcss_shadow(shadow_coords, shadow_position.z);
        }
    }

    let directional_light_contribution = directional_light.color.rgb * light_percent * visibility;

    // Point lights
    var point_light_contribution = vec3<f32>(0.0);
    for (var index = 0u; index < light_count; index++) {
        let light_index = tile_light_indices[tile_index].indices[index];
        let light = point_lights[light_index];
        let light_direction = normalize(input.world_position.xyz - light.position.xyz);
        let light_percent = max(dot(light_direction, input.normal), 0.0);
        let light_distance = length(light.position.xyz - input.world_position.xyz);
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

    let base_color = diffuse_color * global_uniforms.indicator_color;
    let light_contributions = saturate(ambient_light_contribution + directional_light_contribution + point_light_contribution);
    var color = base_color.rgb * light_contributions;

    if (global_uniforms.enhanced_lighting == 0) {
        color = color_balance(color, -0.01, 0.0, 0.0);
    }

    return vec4<f32>(color, base_color.a);
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

fn position_data(vertex_index: u32) -> vec4<f32> {
    switch (vertex_index) {
        case 0u: {
            // upper_left
            return global_uniforms.indicator_positions[0u];
        }
        case 1u: {
            // upper_right
            return global_uniforms.indicator_positions[1u];
        }
        case 2u: {
            // lower_left
            return global_uniforms.indicator_positions[2u];
        }
        case 3u: {
            // lower_left
            return global_uniforms.indicator_positions[2u];
        }
        case 4u: {
            // upper_right
            return global_uniforms.indicator_positions[1u];
        }
        default: {
            // lower_right
            return global_uniforms.indicator_positions[3u];
        }
    }
}

// Optimized version of the following truth table:
//
// vertex_index  x  y
// 0             0  0
// 1             0  1
// 2             1  0
// 3             1  0
// 4             0  1
// 5             1  1
//
// (x,y) are the UV coordinates
fn uv_data(vertex_index: u32) -> vec2<f32> {
    let index = 1u << vertex_index;
    let x = f32((index & 0x2Cu) != 0u);
    let y = f32((index & 0x32u) != 0u);
    return vec2<f32>(x, y);
}

fn normal_data(vertex_index: u32) -> vec3<f32> {
    if (vertex_index < 3) {
        return normalize(cross(
            // upper_right - upper_left, lower_left - upper_left
            (global_uniforms.indicator_positions[1] - global_uniforms.indicator_positions[0]).xyz,
            (global_uniforms.indicator_positions[2] - global_uniforms.indicator_positions[0]).xyz
        ));
    } else {
        return normalize(cross(
            // upper_right - lower_left, lower_right - lower_left
            (global_uniforms.indicator_positions[1] - global_uniforms.indicator_positions[2]).xyz,
            (global_uniforms.indicator_positions[3] - global_uniforms.indicator_positions[2]).xyz
        ));
    }
}

fn get_pcf_shadow(shadow_coords: vec3<f32>, shadow_map_dimensions: vec2<u32>) -> f32 {
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

const FRUSTUM_SIZE: f32 = 400.0;
const LIGHT_WORLD_SIZE: f32 = 5.0;
const LIGHT_SIZE_UV: f32 = LIGHT_WORLD_SIZE / FRUSTUM_SIZE;

const SAMPLE_POINTS_8: array<vec2<f32>, 8> = array<vec2<f32>, 8>(
    vec2<f32>(0.125, -0.375),
    vec2<f32>(-0.125, 0.375),
    vec2<f32>(0.625, 0.125),
    vec2<f32>(-0.375, -0.625),
    vec2<f32>(-0.625, 0.625),
    vec2<f32>(-0.875, -0.125),
    vec2<f32>(0.375, 0.875),
    vec2<f32>(0.875, -0.875),
);

const SAMPLE_POINTS_16: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>(-0.875, -0.875),
    vec2<f32>(-0.750, -0.125),
    vec2<f32>(-0.625, 0.625),
    vec2<f32>(-0.500, -0.375),
    vec2<f32>(-0.375, 0.875),
    vec2<f32>(-0.250, 0.125),
    vec2<f32>(-0.125, -0.625),
    vec2<f32>(0.000, 0.375),
    vec2<f32>(0.125, -0.750),
    vec2<f32>(0.250, 0.500),
    vec2<f32>(0.375, -0.250),
    vec2<f32>(0.500, 0.750),
    vec2<f32>(0.625, 0.000),
    vec2<f32>(0.750, -0.500),
    vec2<f32>(0.875, 0.250),
    vec2<f32>(1.000, -1.000)
);

const SAMPLE_POINTS_32: array<vec2<f32>, 32> = array<vec2<f32>, 32>(
	vec2<f32>(0.06407013, 0.05409927),
	vec2<f32>(0.7366577, 0.5789394),
	vec2<f32>(-0.6270542, -0.5320278),
	vec2<f32>(-0.4096107, 0.8411095),
	vec2<f32>(0.6849564, -0.4990818),
	vec2<f32>(-0.874181, -0.04579735),
	vec2<f32>(0.9989998, 0.0009880066),
	vec2<f32>(-0.004920578, -0.9151649),
	vec2<f32>(0.1805763, 0.9747483),
	vec2<f32>(-0.2138451, 0.2635818),
	vec2<f32>(0.109845, 0.3884785),
	vec2<f32>(0.06876755, -0.3581074),
	vec2<f32>(0.374073, -0.7661266),
	vec2<f32>(0.3079132, -0.1216763),
	vec2<f32>(-0.3794335, -0.8271583),
	vec2<f32>(-0.203878, -0.07715034),
	vec2<f32>(0.5912697, 0.1469799),
	vec2<f32>(-0.88069, 0.3031784),
	vec2<f32>(0.5040108, 0.8283722),
	vec2<f32>(-0.5844124, 0.5494877),
	vec2<f32>(0.6017799, -0.1726654),
	vec2<f32>(-0.5554981, 0.1559997),
	vec2<f32>(-0.3016369, -0.3900928),
	vec2<f32>(-0.5550632, -0.1723762),
	vec2<f32>(0.925029, 0.2995041),
	vec2<f32>(-0.2473137, 0.5538505),
	vec2<f32>(0.9183037, -0.2862392),
	vec2<f32>(0.2469421, 0.6718712),
	vec2<f32>(0.3916397, -0.4328209),
	vec2<f32>(-0.03576927, -0.6220032),
	vec2<f32>(-0.04661255, 0.7995201),
	vec2<f32>(0.4402924, 0.3640312),
);

const SAMPLE_POINTS_64: array<vec2<f32>, 64> = array<vec2<f32>, 64>(
    vec2<f32>(-0.5119625, -0.4827938),
    vec2<f32>(-0.2171264, -0.4768726),
    vec2<f32>(-0.7552931, -0.2426507),
    vec2<f32>(-0.7136765, -0.4496614),
    vec2<f32>(-0.5938849, -0.6895654),
    vec2<f32>(-0.3148003, -0.7047654),
    vec2<f32>(-0.42215, -0.2024607),
    vec2<f32>(-0.9466816, -0.2014508),
    vec2<f32>(-0.8409063, -0.03465778),
    vec2<f32>(-0.6517572, -0.07476326),
    vec2<f32>(-0.1041822, -0.02521214),
    vec2<f32>(-0.3042712, -0.02195431),
    vec2<f32>(-0.5082307, 0.1079806),
    vec2<f32>(-0.08429877, -0.2316298),
    vec2<f32>(-0.9879128, 0.1113683),
    vec2<f32>(-0.3859636, 0.3363545),
    vec2<f32>(-0.1925334, 0.1787288),
    vec2<f32>(0.003256182, 0.138135),
    vec2<f32>(-0.8706837, 0.3010679),
    vec2<f32>(-0.6982038, 0.1904326),
    vec2<f32>(0.1975043, 0.2221317),
    vec2<f32>(0.1507788, 0.4204168),
    vec2<f32>(0.3514056, 0.09865579),
    vec2<f32>(0.1558783, -0.08460935),
    vec2<f32>(-0.0684978, 0.4461993),
    vec2<f32>(0.3780522, 0.3478679),
    vec2<f32>(0.3956799, -0.1469177),
    vec2<f32>(0.5838975, 0.1054943),
    vec2<f32>(0.6155105, 0.3245716),
    vec2<f32>(0.3928624, -0.4417621),
    vec2<f32>(0.1749884, -0.4202175),
    vec2<f32>(0.6813727, -0.2424808),
    vec2<f32>(-0.6707711, 0.4912741),
    vec2<f32>(0.0005130528, -0.8058334),
    vec2<f32>(0.02703013, -0.6010728),
    vec2<f32>(-0.1658188, -0.9695674),
    vec2<f32>(0.4060591, -0.7100726),
    vec2<f32>(0.7713396, -0.4713659),
    vec2<f32>(0.573212, -0.51544),
    vec2<f32>(-0.3448896, -0.9046497),
    vec2<f32>(0.1268544, -0.9874692),
    vec2<f32>(0.7418533, -0.6667366),
    vec2<f32>(0.3492522, 0.5924662),
    vec2<f32>(0.5679897, 0.5343465),
    vec2<f32>(0.5663417, 0.7708698),
    vec2<f32>(0.7375497, 0.6691415),
    vec2<f32>(0.2271994, -0.6163502),
    vec2<f32>(0.2312844, 0.8725659),
    vec2<f32>(0.4216993, 0.9002838),
    vec2<f32>(0.4262091, -0.9013284),
    vec2<f32>(0.2001408, -0.808381),
    vec2<f32>(0.149394, 0.6650763),
    vec2<f32>(-0.09640376, 0.9843736),
    vec2<f32>(0.7682328, -0.07273844),
    vec2<f32>(0.04146584, 0.8313184),
    vec2<f32>(0.9705266, -0.1143304),
    vec2<f32>(0.9670017, 0.1293385),
    vec2<f32>(0.9015037, -0.3306949),
    vec2<f32>(-0.5085648, 0.7534177),
    vec2<f32>(0.9055501, 0.3758393),
    vec2<f32>(0.7599946, 0.1809109),
    vec2<f32>(-0.2483695, 0.7942952),
    vec2<f32>(-0.4241052, 0.5581087),
    vec2<f32>(-0.1020106, 0.6724468),
);

fn get_pcf_pcss_shadow(
    shadow_coords: vec3<f32>,
    pos_from_light: f32
) -> f32 {
    let blocker = find_blocker(shadow_coords.xy, shadow_coords.z, pos_from_light);
    let average_blocker_depth = blocker.x;
    let blocker_count = blocker.y;

    if (blocker_count == 0.0) {
        return 1.0;
    }

    let penumbra = ((average_blocker_depth - shadow_coords.z) * LIGHT_SIZE_UV) / average_blocker_depth;

    return pcf_filter(shadow_coords, penumbra);
}

fn find_blocker(
    uv: vec2<f32>,
    receiver_depth: f32,
    position_from_light: f32
) -> vec2<f32> {
    var blocker_sum = 0.0;
    var blocker_count = 0.0;

    let search_radius = LIGHT_SIZE_UV * position_from_light / position_from_light;

    switch (global_uniforms.shadow_quality) {
        default: {
            // We need at least 16 sample points to get enough coverage for our penumbra.
            for (var i = 0u; i < 16; i++) {
                let offset = SAMPLE_POINTS_16[i] * search_radius;
                let shadow_depth = textureSample(
                    shadow_map,
                    linear_sampler,
                    uv + offset,
                );

                if(receiver_depth < shadow_depth) {
                    blocker_sum += shadow_depth;
                    blocker_count += 1.0;
                }
            }
        }
        case 4u: {
            for (var i = 0u; i < 32; i++) {
                let offset = SAMPLE_POINTS_32[i] * search_radius;
                let shadow_depth = textureSample(
                    shadow_map,
                    linear_sampler,
                    uv + offset,
                );

                if(receiver_depth < shadow_depth) {
                    blocker_sum += shadow_depth;
                    blocker_count += 1.0;
                }
            }
        }
        case 5u: {
            for (var i = 0u; i < 64; i++) {
                let offset = SAMPLE_POINTS_64[i] * search_radius;
                let shadow_depth = textureSample(
                    shadow_map,
                    linear_sampler,
                    uv + offset,
                );

                if(receiver_depth < shadow_depth) {
                    blocker_sum += shadow_depth;
                    blocker_count += 1.0;
                }
            }
        }
    }

    if(blocker_count == 0.0) {
        return vec2<f32>(-1.0, 0.0);
    }

    return vec2(blocker_sum / blocker_count, blocker_count);
}

fn pcf_filter(
    shadow_coords: vec3<f32>,
    filter_radius_uv: f32
) -> f32 {
    var visibility = 0.0;

    switch (global_uniforms.shadow_quality) {
        default: {
            for (var i = 0u; i < 8; i++) {
                let offset = SAMPLE_POINTS_8[i] * filter_radius_uv;
                visibility += textureSampleCompare(
                    shadow_map,
                    shadow_map_sampler,
                    shadow_coords.xy + offset,
                    shadow_coords.z
                );
            }
            return visibility / f32(8);
        }
        case 3u: {
            for (var i = 0u; i < 16; i++) {
                let offset = SAMPLE_POINTS_16[i] * filter_radius_uv;
                visibility += textureSampleCompare(
                    shadow_map,
                    shadow_map_sampler,
                    shadow_coords.xy + offset,
                    shadow_coords.z
                );
            }
            return visibility / f32(16);
        }
        case 4u: {
            for (var i = 0u; i < 32; i++) {
                let offset = SAMPLE_POINTS_32[i] * filter_radius_uv;
                visibility += textureSampleCompare(
                    shadow_map,
                    shadow_map_sampler,
                    shadow_coords.xy + offset,
                    shadow_coords.z
                );
            }
            return visibility / f32(32);
        }
        case 5u: {
            for (var i = 0u; i < 64; i++) {
                let offset = SAMPLE_POINTS_64[i] * filter_radius_uv;
                visibility += textureSampleCompare(
                    shadow_map,
                    shadow_map_sampler,
                    shadow_coords.xy + offset,
                    shadow_coords.z
                );
            }
            return visibility / f32(64);
        }
    }
}

fn linearToNonLinear(linear_depth: f32) -> f32 {
    const NEAR_PLANE = 0.1;
    return NEAR_PLANE / (linear_depth + 1e-7);
}

// Based on "Shadow Techniques from Final Fantasy XVI" by Sammy Fatnassi (2023)
fn get_oriented_bias(normal: vec3<f32>, light_direction: vec3<f32>) -> f32 {
    let bias = 0.002;
    let is_facing_light = dot(normal, light_direction) > 0.0;
    // sic! We use reverse Z projection!
    return select(-bias, bias, is_facing_light);
}
