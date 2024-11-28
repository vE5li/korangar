struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    screen_size: vec2<u32>,
    interface_size: vec2<u32>,
    pointer_position: vec2<u32>,
    animation_timer: f32,
    day_timer: f32,
    water_level: f32,
    point_light_count: u32,
}

struct InstanceData {
    color: vec4<f32>,
    corner_radius: vec4<f32>,
    screen_clip: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    rectangle_type: u32,
    texture_index: i32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
    @location(1) texture_coordinates: vec2<f32>,
    @location(2) instance_index: u32,
}

@group(0) @binding(0) var<uniform> global_uniforms: GlobalUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(0) @binding(2) var linear_sampler: sampler;
@group(1) @binding(0) var<storage, read> instance_data: array<InstanceData>;
@group(1) @binding(1) var font_atlas: texture_2d<f32>;
@group(2) @binding(0) var texture: texture_2d<f32>;

// Because of how sampling works, we need to add a bit breathing room
// for the SDF function and then need to compensate for it.
const BREATHING_ROOM = 0.5;
const BORDER_THRESHOLD = 2.0;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let instance = instance_data[instance_index];
    let vertex = vertex_data(vertex_index);

    let pixel_size = vec2<f32>(1.0 / f32(global_uniforms.interface_size.x), 1.0 / f32(global_uniforms.interface_size.y));
    let size_adjustment = select(vec2<f32>(0.0), (BREATHING_ROOM * 2.0) * pixel_size, any(instance.corner_radius != vec4<f32>(0.0)));

    let adjusted_size = instance.screen_size + size_adjustment;
    let clip_size = adjusted_size * 2.0;
    let position = screen_to_clip_space(instance.screen_position) + vertex.xy * clip_size;

    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.fragment_position = clip_to_screen_space(position);
    output.texture_coordinates = instance.texture_position + vertex.zw * instance.texture_size;
    output.instance_index = instance_index;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instance_data[input.instance_index];

    let clip_adjustment = select(vec4<f32>(0.0), vec4<f32>(-0.5, -0.5, 0.5, 0.5), any(instance.corner_radius != vec4<f32>(0.0)));
    let adjusted_clip = instance.screen_clip + clip_adjustment;

    if (input.position.x < adjusted_clip.x || input.position.y < adjusted_clip.y ||
        input.position.x > adjusted_clip.z || input.position.y > adjusted_clip.w) {
        return vec4<f32>(0.0);
    }

    var color: vec4<f32> = instance.color;

    switch (instance.rectangle_type) {
        case 1u: {
            // Sprite (linear filtering)
            color *= textureSample(texture, linear_sampler, input.texture_coordinates);
        }
        case 2u: {
            // Sprite (nearest filtering)
            color *= textureSample(texture, nearest_sampler, input.texture_coordinates);
        }
        case 3u: {
            // Text (coverage)
            color *= textureSample(font_atlas, linear_sampler, input.texture_coordinates).r;
        }
        default: {}
    }

    return rectangle_with_rounded_edges(
        instance.corner_radius,
        instance.screen_position,
        instance.screen_size,
        input.fragment_position,
        color
    );
}

fn rectangle_with_rounded_edges(
    corner_radii: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    fragment_position: vec2<f32>,
    color: vec4<f32>,
) -> vec4<f32> {
    if (all(corner_radii == vec4<f32>(0.0))) {
        return color;
    }

    // Convert normalized screen space coordinates to pixel space.
    let interface_size = vec2<f32>(global_uniforms.interface_size);
    let position = fragment_position * interface_size;
    let origin = (screen_position * interface_size) - vec2<f32>(BREATHING_ROOM);
    let size = (screen_size * interface_size) + vec2<f32>(BREATHING_ROOM * 2.0);

    // Calculate position relative to rectangle center.
    let half_size = size * 0.5;
    let rectangle_center = origin + half_size;
    let relative_position = position - rectangle_center;

    // Determine which corner radius to use based on the quadrant this fragment is in.
    let is_right = relative_position.x > 0.0;
    let is_bottom = relative_position.y > 0.0;
    let radii_pair = select(corner_radii.xy, corner_radii.zw, is_bottom);
    let corner_radius = select(radii_pair.x, radii_pair.y, is_right);

    if (corner_radius == 0.0) {
        return color;
    }

    // We multi-sample the edges of a rectangle to get the best possible anti-aliasing.
    let distance = rectangle_sdf(relative_position, half_size, corner_radius);

    var alpha: f32 = step(0.0, -distance);

    if (abs(distance) <= BORDER_THRESHOLD) {
        var total = alpha;
        for (var index = 0u; index < 8u; index++) {
            let offset = SAMPLE_OFFSETS[index];
            let sample_distance = rectangle_sdf(relative_position + offset, half_size, corner_radius);
            total += step(0.0, -sample_distance);
        }
        alpha = total * (1.0/9.0);
    }

    return color * alpha;
}

// 8-point Poisson Disk pattern
const SAMPLE_OFFSETS: array<vec2<f32>, 8> = array<vec2<f32>, 8>(
    vec2<f32>( 0.924,  0.382) * 0.5,
    vec2<f32>( 0.382,  0.924) * 0.5,
    vec2<f32>(-0.382,  0.924) * 0.5,
    vec2<f32>(-0.924,  0.382) * 0.5,
    vec2<f32>(-0.924, -0.382) * 0.5,
    vec2<f32>(-0.382, -0.924) * 0.5,
    vec2<f32>( 0.382, -0.924) * 0.5,
    vec2<f32>( 0.924, -0.382) * 0.5
);

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

fn clip_to_screen_space(ndc: vec2<f32>) -> vec2<f32> {
    let u = (ndc.x + 1.0) / 2.0;
    let v = (1.0 - ndc.y) / 2.0;
    return vec2<f32>(u, v);
}

// Calculation based on:
// "Leveraging Rust and the GPU to render user interfaces at 120 FPS"
// https://zed.dev/blog/videogame
fn rectangle_sdf(
    relative_position: vec2<f32>,
    half_size: vec2<f32>,
    corner_radius: f32
) -> f32 {
    let shrunk_corner_position = half_size - corner_radius;
    let pixel_to_shrunk_corner = max(vec2<f32>(0.0), abs(relative_position) - shrunk_corner_position);
    return length(pixel_to_shrunk_corner) - corner_radius + BREATHING_ROOM;
}
