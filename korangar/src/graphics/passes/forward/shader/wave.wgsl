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

struct WaterWaveUniforms {
    texture_repeat_rcp: f32,
    waveform_phase_shift: f32,
    waveform_amplitude: f32,
    waveform_frequency: f32,
    water_opacity: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct WboitOutput {
    @location(1) accumulation: vec4<f32>,
    @location(2) revealage: f32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(3) var texture_sampler: sampler;
@group(1) @binding(0) var<uniform> directional_light: DirectionalLightUniforms;
@group(2) @binding(0) var<uniform> water_wave_uniforms: WaterWaveUniforms;
@group(2) @binding(1) var texture: texture_2d<f32>;

// The reciprocal size of a tile in world units.
const MAP_TILE_SIZE_RCP: f32 = 1.0 / 10.0;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) @interpolate(flat) grid: vec2<i32>
) -> VertexOutput {
    let distance_from_wave_crest = getDistanceFromSamplingOrigin(vertex_index);
    let phase_shift = phaseShiftAtSampledPoint(grid.x, grid.y, distance_from_wave_crest);
	var wave_height = sampleWaveHeight(phase_shift);
    let water_height = position.y + wave_height;

    let world_position = vec4<f32>(position.x, water_height, position.z, 1.0);
    let normal = calculateWaveNormal(phase_shift, grid.x, grid.y);;

    var output: VertexOutput;
    output.position = global_uniforms.view_projection * world_position;
    output.world_position = world_position.xyz;
    output.normal = normal;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> WboitOutput {
    let color = calculate_wave_color(input.world_position, input.normal);

    // Equation from https://casual-effects.blogspot.com/2015/03/implemented-weighted-blended-order.html
    let depth = input.position.z;
    let weight = clamp(pow(min(1.0, color.a * 10.0) + 0.01, 3.0) * 1e8 * pow(depth * 0.9, 3.0), 1e-2, 3e3);

    var output = WboitOutput();
    output.accumulation = color * weight;
    output.revealage = color.a;
    return output;
}

fn getDistanceFromSamplingOrigin(vertexIndex: u32) -> i32 {
	// Vertices are pushed in order SW, SE, NW, NE (repeating pattern for the entire grid)
	let corner = vertexIndex % 4;

    switch (corner) {
        // SW
        case 0: { return -1; }
        // NE
        case 3: {return 1; }
        // SE & NW
        default: { return 0; }
    }
}

fn phaseShiftAtSampledPoint(grid_u: i32, grid_v: i32, relative_distance_from_wave_crest: i32) -> f32 {
	let phase_shift_at_sampling_origin = i32(water_wave_uniforms.waveform_phase_shift);
	let phaseDeltaToSampledPoint = grid_u + grid_v + relative_distance_from_wave_crest;
	let phaseShiftAtSampledPoint = phaseDeltaToSampledPoint * i32(water_wave_uniforms.waveform_frequency);
	let phase_shift_in_degrees = (phase_shift_at_sampling_origin + phaseShiftAtSampledPoint) % 360;
	return radians(f32(phase_shift_in_degrees));
}

fn sampleWaveHeight(phase_shift: f32) -> f32 {
	return sin(phase_shift) * water_wave_uniforms.waveform_amplitude;
}

fn calculateWaveNormal(phase_shift: f32, grid_u: i32, grid_v: i32) -> vec3<f32> {
    let phase_change_per_grid_unit = radians(f32(i32(water_wave_uniforms.waveform_frequency)));

    let dh_dphase = cos(phase_shift) * water_wave_uniforms.waveform_amplitude;

    let dphase_dx = phase_change_per_grid_unit;
    let dphase_dz = phase_change_per_grid_unit;

    let dydx = dh_dphase * dphase_dx;
    let dydz = dh_dphase * dphase_dz;

    let normal = vec3<f32>(dydx, 1.0, dydz);

    return normalize(normal);
}

fn calculate_wave_color(world_position: vec3<f32>, normal: vec3<f32>) -> vec4<f32> {
    let tile_x = world_position.x * MAP_TILE_SIZE_RCP;
    let tile_z = world_position.z * MAP_TILE_SIZE_RCP;
    let texture_coordinates = vec2<f32>(tile_x, tile_z) * water_wave_uniforms.texture_repeat_rcp;

    var base_color = textureSample(texture, texture_sampler, texture_coordinates);
    var final_color = base_color.rgb;

    if (global_uniforms.enhanced_lighting != 0) {
        // Directional light
        let light_percent = clamp(dot(normalize(-directional_light.direction.xyz), normal), 0.0, 1.0);
        let directional_light = light_percent * directional_light.color.rgb;

        final_color *= global_uniforms.ambient_color.rgb + directional_light.rgb;
    }

    final_color *= water_wave_uniforms.water_opacity;

    return vec4<f32>(final_color, water_wave_uniforms.water_opacity);
}
