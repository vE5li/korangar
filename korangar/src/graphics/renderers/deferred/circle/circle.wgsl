struct Constants {
    position: vec4<f32>,
    color: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    size: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
}

var<push_constant> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertex = vertex_data(vertex_index);
    let clip_size = constants.screen_size * 2.0;
    let position = screen_to_clip_space(constants.screen_position) + vertex.xy * clip_size;

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.fragment_position = vec2<f32>(vertex);
    return output;
}

@fragment
fn fs_main(
    @location(0) fragment_position: vec2<f32>,
) -> @location(0) vec4<f32> {
    let distance_from_center = distance(fragment_position.xy, vec2<f32>(0.5, -0.5));
    let scaling_factor = constants.screen_size.y / 2.0;
    let intensity = clamp(gaussian_peak(distance_from_center, scaling_factor), 0.0, 1.0);

    return vec4<f32>(constants.color.rgb, intensity);
}

fn gaussian_peak(x: f32, scaling_factor: f32) -> f32 {
    // The variance defines how steep the peek is.
    // A larger number will make it fuzzier, a smaller number will make the circle sharper.
    let variance = 0.0002 / scaling_factor;

    // The expected value defines the position of the peek.
    // Since x ranges from 0.0 at the center to 0.5 at the closest edge we want a little less than 0.5 to not cut off the edges.
    let expected_value = 0.5 - variance * 10.0;

    return exp(-(pow(x - expected_value, 2.0)) / (2.0 * variance * variance));
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
