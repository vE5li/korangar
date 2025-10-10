#language slang 2026

import globals;
import forward;
import sdsm;
import matrix;

struct WaterWaveUniforms {
    var texture_repeat_rcp: float;
    var waveform_phase_shift: float;
    var waveform_amplitude: float;
    var waveform_frequency: float;
    var water_opacity: float;
}

struct WaveVertexInput {
    uint vertex_index: SV_VulkanVertexID;
    [[vk::location(0)]] float3 position;
    [[vk::location(1)]] int2 grid;
}

struct WaveVertexOutput {
    float4 position: SV_Position;
    [[vk::location(0)]] var world_position: float3;
    [[vk::location(1)]] var normal: float3;
}

static const var TILE_SIZE: uint = 16;
// The reciprocal size of a tile in world units.
static const var MAP_TILE_SIZE_RCP: float = 1.0 / 10.0;

[[vk::binding(0, 0)]] var global_uniforms: ConstantBuffer<GlobalUniforms>;
[[vk::binding(3, 0)]] var texture_sampler: SamplerState;
[[vk::binding(0, 1)]] var directional_light: ConstantBuffer<DirectionalLightUniforms>;
[[vk::binding(0, 2)]] var water_wave_uniforms: ConstantBuffer<WaterWaveUniforms>;
[[vk::binding(1, 2)]] var texture: Texture2D;

[[shader("vertex")]]
func vs_main(input: WaveVertexInput) -> WaveVertexOutput {
    let distance_from_wave_crest = getDistanceFromSamplingOrigin(input.vertex_index);
    let phase_shift = phaseShiftAtSampledPoint(input.grid.x, input.grid.y, distance_from_wave_crest);
    var wave_height = sampleWaveHeight(phase_shift);
    let water_height = input.position.y + wave_height;

    let world_position = float4(input.position.x, water_height, input.position.z, 1.0);
    let normal = calculateWaveNormal(phase_shift, input.grid.x, input.grid.y);

    var output: WaveVertexOutput;
    output.position = mul(global_uniforms.view_projection, world_position);
    output.world_position = world_position.xyz;
    output.normal = normal;
    return output;
}

[[shader("pixel")]]
func fs_main(input: WaveVertexOutput) -> WboitOutput {
    let color = calculate_wave_color(input.world_position, input.normal);

    // Equation from https://casual-effects.blogspot.com/2015/03/implemented-weighted-blended-order.html
    let depth = input.position.z;
    let weight = clamp(pow(min(1.0, color.a * 10.0) + 0.01, 3.0) * 1e8 * pow(depth * 0.9, 3.0), 1e-2, 3e3);

    var output: WboitOutput;
    output.accumulation = color * weight;
    output.revealage = color.a;
    return output;
}

[ForceInline]
func getDistanceFromSamplingOrigin(vertexIndex: uint) -> int {
    // Vertices are pushed in order SW, SE, NW, NE (repeating pattern for the entire grid).
    let corner = vertexIndex % 4;

    switch (corner) {
        // SW
        case 0: { return -1; }
        // NE
        case 3: { return 1; }
        // SE & NW
        default: { return 0; }
    }
}

[ForceInline]
func phaseShiftAtSampledPoint(grid_u: int, grid_v: int, relative_distance_from_wave_crest: int) -> float {
    let phase_shift_at_sampling_origin = int(water_wave_uniforms.waveform_phase_shift);
    let phaseDeltaToSampledPoint = grid_u + grid_v + relative_distance_from_wave_crest;
    let phaseShiftAtSampledPoint = phaseDeltaToSampledPoint * int(water_wave_uniforms.waveform_frequency);
    let phase_shift_in_degrees = (phase_shift_at_sampling_origin + phaseShiftAtSampledPoint) % 360;
    return radians(float(phase_shift_in_degrees));
}

[ForceInline]
func sampleWaveHeight(phase_shift: float) -> float {
    return sin(phase_shift) * water_wave_uniforms.waveform_amplitude;
}

[ForceInline]
func calculateWaveNormal(phase_shift: float, grid_u: int, grid_v: int) -> float3 {
    let phase_change_per_grid_unit = radians(float(int(water_wave_uniforms.waveform_frequency)));

    let dh_dphase = cos(phase_shift) * water_wave_uniforms.waveform_amplitude;

    let dphase_dx = phase_change_per_grid_unit;
    let dphase_dz = phase_change_per_grid_unit;

    let dydx = dh_dphase * dphase_dx;
    let dydz = dh_dphase * dphase_dz;

    let normal = float3(dydx, 1.0, dydz);

    return normalize(normal);
}

[ForceInline]
func calculate_wave_color(world_position: float3, normal: float3) -> float4 {
    let tile_x = world_position.x * MAP_TILE_SIZE_RCP;
    let tile_z = world_position.z * MAP_TILE_SIZE_RCP;
    let texture_coordinates = float2(tile_x, tile_z) * water_wave_uniforms.texture_repeat_rcp;

    var base_color = texture.Sample(texture_sampler, texture_coordinates);
    var final_color = base_color.rgb;

    if (global_uniforms.enhanced_lighting != 0) {
        // Directional light
        let light_percent = saturate(dot(normalize(-directional_light.direction.xyz), normal));
        let directional_light_color = light_percent * directional_light.color.rgb;

        final_color *= global_uniforms.ambient_color.rgb + directional_light_color;
    }

    final_color *= water_wave_uniforms.water_opacity;

    return float4(final_color, water_wave_uniforms.water_opacity);
}
