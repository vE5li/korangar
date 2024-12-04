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
}

struct DirectionalLightUniforms {
    view_projection: mat4x4<f32>,
    color: vec4<f32>,
    direction: vec4<f32>,
}

struct WaterWaveUniforms {
    texture_repeat: f32,
    water_level: f32,
    wave_amplitude: f32,
    wave_speed: f32,
    wave_length: f32,
    water_opacity: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(2) var linear_sampler: sampler;
@group(0) @binding(3) var texture_sampler: sampler;
@group(1) @binding(0) var<uniform> directional_light: DirectionalLightUniforms;
@group(2) @binding(0) var<uniform> water_wave_uniforms: WaterWaveUniforms;
@group(2) @binding(1) var texture: texture_2d<f32>;
@group(3) @binding(0) var depth_texture: texture_depth_2d;

// Higher values might be needed if the view angle is smaller (against the horizon).
const MAX_STEPS: i32 = 4;
// Smaller value improves the "resolution" at the edges of obejcts.
const EPSILON: f32 = 0.05;
// The size of a tile in world units.
const MAP_TILE_SIZE: f32 = 10.0;
// Earth's gravitational acceleration (m/s²).
const GRAVITY: f32 = 9.8;
// 2π = τ
const TAU: f32 = 6.28318548;
// Pre-normalized direction vector (-1/√2, -1/√2)
const NORMALIZED_WAVE_DIRECTION: vec2<f32> = vec2<f32>(-0.707107, -0.707107);

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    var output: VertexOutput;
    output.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    output.uv = uv;
    return output;
}

var<private> wave_frequency: f32;
var<private> wave_phase_speed: f32;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = input.uv;
    let scene_depth = textureSample(depth_texture, linear_sampler, uv);
    let scene_position = reconstruct_world_position(input.uv, scene_depth);

    var color = vec4<f32>(0.0);

    // A early exit if the depth of the scene is above the water (with a little bit of headroom).
    // This will only work correctly for a top down view.
    let max_wave_height = (-water_wave_uniforms.water_level + water_wave_uniforms.wave_amplitude) + 0.0001;
    if (scene_position.y > max_wave_height) {
        discard;
    } else {
        let clip_space_uv = screen_to_clip_space(uv);
        let near_point = vec4<f32>(clip_space_uv, 1.0, 1.0);
        let far_point = vec4<f32>(clip_space_uv, 0.00001, 1.0);

        let world_near = global_uniforms.inverse_view_projection * near_point;
        let world_far = global_uniforms.inverse_view_projection * far_point;

        let normalized_world_near = world_near.xyz / world_near.w;
        let normalized_world_far = world_far.xyz / world_far.w;

        let ray_direction = normalize(normalized_world_far - normalized_world_near);
        let ray_origin = global_uniforms.camera_position.xyz;
        let max_distance = distance(ray_origin, scene_position);

        wave_frequency = TAU / water_wave_uniforms.wave_length;
        wave_phase_speed = sqrt(GRAVITY / wave_frequency);

        let distance = find_wave_intersection(ray_origin, ray_direction, max_distance, max_wave_height);

        if (distance < 0.0 || distance > max_distance) {
            discard;
        } else {
            let hit_position = ray_origin + ray_direction * distance;
            color = calculate_wave_color(hit_position);
        }
    }

    return color;
}

/// Finds the intersection point between a ray and a wave surface using the Secant method.
/// This approach is significantly more efficient than ray marching because:
///
/// 1. It uses a highly accurate initial guess by starting at the maximum wave height plane
/// 2. Leverages the Secant method's superlinear convergence (order ≈1.618)
/// 3. Typically converges in 1-3 iterations versus up to 16 iterations with ray marching
/// 4. Adapts step size based on ray angle to prevent missing wave periods
fn find_wave_intersection(ray_origin: vec3<f32>, ray_direction: vec3<f32>, max_distance: f32, max_wave_height: f32) -> f32 {
    if (water_wave_uniforms.wave_amplitude == 0.0) {
        // We found a flat water plane.
        return ray_plane_intersection(ray_origin, ray_direction, -water_wave_uniforms.water_level);
    }

    if (max_distance >= 1e30) {
        // We look outside of the scene.
        return -1.0;
    }

    let initial_guess = ray_plane_intersection(ray_origin, ray_direction, max_wave_height);

    if (initial_guess < 0.0 || initial_guess > max_distance) {
        // Ray points away from the wave surface or intersect beyond scene geometry.
        return -1.0;
    }

    var x1 = initial_guess;
    var f1 = wave_sdf(ray_origin + ray_direction * x1);

    let step_size = water_wave_uniforms.wave_amplitude * 0.25 / abs(ray_direction.y);
    var x0 = x1 - step_size;
    var f0 = wave_sdf(ray_origin + ray_direction * x0);

    for(var step = 0; step < MAX_STEPS; step++) {
        let x2 = x1 - f1 * (x1 - x0) / (f1 - f0);
        x0 = x1;
        x1 = x2;

        if(x1 > max_distance || abs(x1 - x0) < EPSILON) {
            // We either hit the scene geometry or reached the wave.
            break;
        }

        f0 = f1;
        f1 = wave_sdf(ray_origin + ray_direction * x1);
    }

    return x1;
}

/// Calculates the intersection of a ray with a horizontal plane (water surface).
///
/// It simplifies the general plane-ray intersection formula since we know:
///    - The plane normal is always (0,1,0)
///    - We only view from above water
///
/// This removes the need for dot product calculations and normalizations.
fn ray_plane_intersection(
    ray_origin: vec3<f32>,
    ray_direction: vec3<f32>,
    plane_height: f32
) -> f32 {
    // If ray is going up or parallel to the water surface, it will never hit.
    if (ray_direction.y >= -0.0001) {
        return -1.0;
    }
    return (plane_height - ray_origin.y) / ray_direction.y;
}

fn reconstruct_world_position(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let clip_space_position = vec4<f32>(screen_to_clip_space(uv), depth, 1.0);
    let world_space_positon = global_uniforms.inverse_view_projection * clip_space_position;
    return world_space_positon.xyz / world_space_positon.w;
}

/// Calculates the signed distance from the position to the wave.
fn wave_sdf(position: vec3<f32>) -> f32 {
    let wave_height = -water_wave_uniforms.water_level + gerstner_wave(position);
    return position.y - wave_height;
}

/// Calculates a simplified vertical displacement for water wave simulation using
/// a modified Gerstner wave approach.
///
/// This is a basic approximation of water wave behavior. While true Gerstner waves
/// model circular motion of water particles (resulting in both horizontal and vertical
/// displacement), this function only calculates vertical displacement using a
/// sinusoidal wave pattern.
///
/// The approximation uses physical properties like gravity and wavelength to determine
/// wave behavior, but simplifies the full Gerstner wave equations for performance.
///
/// Formula: A * sin(k * (dot(D, P) - c * t))
//
/// where:
/// - A: Wave amplitude
/// - k: Wave frequency (2π/wavelength)
/// - D: Wave direction (normalized)
/// - P: Surface position
/// - c: Phase speed (sqrt(g/k))
/// - t: Time
///
/// The phase speed (c) is determined by gravity and wavelength: c = sqrt(g/k)
/// This means longer waves move faster than shorter ones.
fn gerstner_wave(position: vec3<f32>) -> f32 {
    let wave_phase = wave_frequency * (
        dot(NORMALIZED_WAVE_DIRECTION, position.xz) -
        wave_phase_speed * global_uniforms.animation_timer * water_wave_uniforms.wave_speed
    );
    return water_wave_uniforms.wave_amplitude * sin(wave_phase);
}

fn calculate_wave_color(hit_position: vec3<f32>) -> vec4<f32> {
    let tile_x = hit_position.x / MAP_TILE_SIZE;
    let tile_z = hit_position.z / MAP_TILE_SIZE;

    let texture_coordinates = vec2<f32>(
        fract(tile_x / water_wave_uniforms.texture_repeat),
        fract(tile_z / water_wave_uniforms.texture_repeat)
    );

    var base_color = textureSample(texture, texture_sampler, texture_coordinates);

    let normal = calculate_wave_normal(hit_position);

    // Directional light
    let light_percent = clamp(dot(normalize(-directional_light.direction.xyz), normal), 0.0, 1.0);
    let bias = clamp(0.0025 * tan(acos(light_percent)), 0.0, 0.0005);
    let directional_light = light_percent * directional_light.color.rgb * base_color.rgb;

    // Combine base color, ambient light and directional light
    var final_color = base_color.rgb * global_uniforms.ambient_color.rgb + directional_light.rgb;
    final_color *= water_wave_uniforms.water_opacity;

    return vec4<f32>(final_color, water_wave_uniforms.water_opacity);
}

fn calculate_wave_normal(position: vec3<f32>) -> vec3<f32> {
    let wave_phase = wave_frequency * (
        dot(NORMALIZED_WAVE_DIRECTION, position.xz) -
        wave_phase_speed * global_uniforms.animation_timer * water_wave_uniforms.wave_speed
    );

    let derivative = water_wave_uniforms.wave_amplitude * wave_frequency * cos(wave_phase);

    return normalize(vec3<f32>(
        -NORMALIZED_WAVE_DIRECTION.x * derivative,
        1.0,
        -NORMALIZED_WAVE_DIRECTION.y * derivative
    ));
}

fn clip_to_screen_space(ndc: vec2<f32>) -> vec2<f32> {
    let u = (ndc.x + 1.0) / 2.0;
    let v = (1.0 - ndc.y) / 2.0;
    return vec2<f32>(u, v);
}

fn screen_to_clip_space(screen_coords: vec2<f32>) -> vec2<f32> {
    let x = (screen_coords.x * 2.0) - 1.0;
    let y = -(screen_coords.y * 2.0) + 1.0;
    return vec2<f32>(x, y);
}
