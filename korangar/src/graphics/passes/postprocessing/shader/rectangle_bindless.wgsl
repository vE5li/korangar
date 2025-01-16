struct InstanceData {
    color: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    rectangle_type: u32,
    texture_index: i32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinates: vec2<f32>,
    @location(1) instance_index: u32,
}

@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(1) @binding(1) var msdf_font_map: texture_2d<f32>;
@group(1) @binding(2) var textures: binding_array<texture_2d<f32>>;

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
    output.texture_coordinates = instance.texture_position + vertex.zw * instance.texture_size;
    output.instance_index = instance_index;
    return output;
}

const EDGE_VALUE: f32 = 0.5;
/// We use a range of 6 px when creating SDF and MSDF (This seems to be pxrange of 6 in MSDFGen and range of 3 in SDFMaker).
const PXRANGE: f32 = 6.0;
const SHADOW_SPREAD: f32 = 0.45;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instance_data[input.instance_index];

    var color: vec4<f32> = instance.color;

    switch (instance.rectangle_type) {
        case 1u: {
            // Sprite (linear filtering)
            color *= textureSample(textures[instance.texture_index], linear_sampler, input.texture_coordinates);
        }
        case 2u: {
            // Sprite (nearest filtering)
            color *= textureSample(textures[instance.texture_index], nearest_sampler, input.texture_coordinates);
        }
        case 3u: {
            // SDF
            let texture = textures[instance.texture_index];
            let distance = textureSample(texture, linear_sampler, input.texture_coordinates).r - EDGE_VALUE;
            let texture_size = vec2<f32>(textureDimensions(texture));
            color = calculate_shadowed_sdf(distance, texture_size, input.texture_coordinates, color);
        }
        case 4u: {
            // Text
            let distances = textureSample(msdf_font_map, linear_sampler, input.texture_coordinates);
            let distance = median(distances.r, distances.g, distances.b) - EDGE_VALUE;
            let texture_size = vec2<f32>(textureDimensions(msdf_font_map));
            color = calculate_shadowed_sdf(distance, texture_size, input.texture_coordinates, color);
        }
        default: {}
    }

    return color;
}

fn calculate_shadowed_sdf(
    distance: f32,
    texture_size: vec2<f32>,
    texture_coordinates: vec2<f32>,
    color: vec4<f32>
) -> vec4<f32> {
    let unit_range = vec2<f32>(PXRANGE) / texture_size;
    let screen_texture_size = vec2<f32>(1.0) / fwidth(texture_coordinates);
    let screen_px_range = max(EDGE_VALUE * dot(unit_range, screen_texture_size), 1.0);

    let shadow_distance = distance + SHADOW_SPREAD;
    let shadow_alpha = saturate(shadow_distance * screen_px_range);
    let shadow_color = vec4<f32>(0.0, 0.0, 0.0, shadow_alpha);

    let text_alpha = saturate(distance * screen_px_range + EDGE_VALUE);
    let text_color = color * text_alpha;

    return mix(shadow_color, text_color, text_alpha);
}

// Optimized version of the following truth table:
//
// vertex_index  x  y  z  w
// 0             0  0  0  0
// 1             1  0  1  0
// 2             1 -1  1  1
// 3             1 -1  1  1
// 4             0 -1  0  1
// 5             0  0  0  0
//
// (x,y) are the vertex position
// (z,w) are the UV coordinates
fn vertex_data(vertex_index: u32) -> vec4<f32> {
    let index = 1u << vertex_index;
    let x = f32((index & 0xEu) != 0u);
    let y = f32((index & 0x1Cu) != 0u);
    return vec4<f32>(x, -y, x, y);
}

fn screen_to_clip_space(screen_coords: vec2<f32>) -> vec2<f32> {
    let x = (screen_coords.x * 2.0) - 1.0;
    let y = -(screen_coords.y * 2.0) + 1.0;
    return vec2<f32>(x, y);
}

fn median(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}
