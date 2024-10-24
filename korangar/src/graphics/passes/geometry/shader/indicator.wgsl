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

struct IndicatorUniforms {
    upper_left: vec4<f32>,
    upper_right: vec4<f32>,
    lower_left: vec4<f32>,
    lower_right: vec4<f32>,
    color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) normal: vec4<f32>,
}

struct FragmentOutput {
    @location(0) fragment_color: vec4<f32>,
    @location(1) fragment_normal: vec4<f32>,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    output.position = global_uniforms.view_projection * position_data(vertex_index);
    output.texture_coordinates = uv_data(vertex_index);
    output.normal = vec4<f32>(normal_data(vertex_index), 1.0);
    return output;
}

@fragment
fn fs_main(
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) normal: vec4<f32>,
) -> FragmentOutput {
    let texel_color = textureSample(texture, nearest_sampler, texture_coordinates);

    if (texel_color.a < 0.1) {
        discard;
    }

    var output: FragmentOutput;
    output.fragment_color = vec4<f32>(texel_color.rgb * global_uniforms.indicator_color.rgb, texel_color.a);
    output.fragment_normal = normal;
    return output;
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
