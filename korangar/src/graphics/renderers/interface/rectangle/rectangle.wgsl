struct Constants {
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    screen_clip: vec4<f32>,
    corner_radius: vec4<f32>,
    color: vec4<f32>,
    aspect_ratio: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
}

var<push_constant> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let vertex = vertex_data(vertex_index);

    // UV (0.0 to 2.0) to NDC
    let x = constants.screen_position.x - 1.0 + vertex.x * constants.screen_size.x;
    let y = -constants.screen_position.y + 1.0 - vertex.y * constants.screen_size.y;

    var output: VertexOutput;
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.fragment_position = vertex;
    return output;
}

@fragment
fn fs_main(
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>
) -> @location(0) vec4<f32> {
    var coords = fragment_position * constants.screen_size;
    var screen_size = constants.screen_size;

    coords.x /= constants.aspect_ratio;
    screen_size.x /= constants.aspect_ratio;

    // top-left
    if (length(coords - constants.corner_radius.x) > constants.corner_radius.x && coords.x < constants.corner_radius.x &&
        coords.y < constants.corner_radius.x) {
        discard;
    }

    // top-right
    if (length(coords - vec2<f32>(screen_size.x - constants.corner_radius.y, constants.corner_radius.y)) > constants.corner_radius.y &&
        screen_size.x - coords.x < constants.corner_radius.y && coords.y < constants.corner_radius.y) {
        discard;
    }

    // bottom-right
    if (length(coords - screen_size + constants.corner_radius.z) > constants.corner_radius.z &&
        screen_size.x - coords.x < constants.corner_radius.z && screen_size.y - coords.y < constants.corner_radius.z) {
        discard;
    }

    // bottom-left
    if (length(coords - vec2<f32>(constants.corner_radius.w, screen_size.y - constants.corner_radius.w)) > constants.corner_radius.w &&
        coords.x < constants.corner_radius.w && screen_size.y - coords.y < constants.corner_radius.w) {
        discard;
    }

    if (position.x < constants.screen_clip.x || position.y < constants.screen_clip.y ||
        position.x > constants.screen_clip.z || position.y > constants.screen_clip.w) {
        discard;
    }

    return constants.color;
}

// Optimized version of the following truth table:
//
// vertex_index  x  y
// 0             0  0
// 1             1  0
// 2             1  1
// 3             1  1
// 4             0  1
// 5             0  0
//
// (x,y) are the vertex position
fn vertex_data(vertex_index: u32) -> vec2<f32> {
    let index = 1u << vertex_index;
    let x = f32((index & 0xEu) != 0u);
    let y = f32((index & 0x1Cu) != 0u);
    return vec2<f32>(x, y);
}
