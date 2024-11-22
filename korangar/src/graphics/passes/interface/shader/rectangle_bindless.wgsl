struct GlobalUniforms {
    view_projection: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    indicator_positions: mat4x4<f32>,
    indicator_color: vec4<f32>,
    ambient_color: vec4<f32>,
    screen_size: vec2<u32>,
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
    linear_filtering: u32,
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
@group(1) @binding(1) var textures: binding_array<texture_2d<f32>>;
@group(1) @binding(2) var font_atlas: texture_2d<f32>;

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
    output.fragment_position = clip_to_screen_space(position);
    output.texture_coordinates = instance.texture_position + vertex.zw * instance.texture_size;
    output.instance_index = instance_index;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instance_data[input.instance_index];

    if (input.position.x < instance.screen_clip.x || input.position.y < instance.screen_clip.y ||
        input.position.x > instance.screen_clip.z || input.position.y > instance.screen_clip.w) {
        return vec4<f32>(0.0);
    }

    switch (instance.rectangle_type) {
        case 0u: {
            return draw_solid(instance, input.fragment_position);
        }
        case 1u: {
            return draw_sprite(instance, input.texture_coordinates);
        }
        default: {
            return draw_text(instance, input.texture_coordinates);
        }
    }
}

fn draw_solid(
    instance: InstanceData,
    fragment_position: vec2<f32>
) -> vec4<f32> {
    let corner_radii = instance.corner_radius;

    if (all(corner_radii == vec4<f32>(0.0))) {
        return instance.color;
    }

    // Convert normalized screen space coordinates to pixel space.
    let window_size = vec2<f32>(global_uniforms.screen_size);
    let position = fragment_position * window_size;
    let screen_position = instance.screen_position * window_size;
    let screen_size = instance.screen_size * window_size;

    // Calculate position relative to rectangle center.
    let half_screen_size = screen_size * 0.5;
    let rectangle_center = screen_position + half_screen_size;
    let relative_position = position - rectangle_center;

    // Determine which corner radius to use based on the quadrant this fragment is in.
    let is_right = relative_position.x > 0.0;
    let is_bottom = relative_position.y > 0.0;
    let radii_pair = select(corner_radii.xy, corner_radii.zw, is_bottom);
    let corner_radius = select(radii_pair.x, radii_pair.y, is_right);

    if (corner_radius == 0.0) {
        return instance.color;
    }

    var total = 0.0;
    for (var i = 0u; i < 16u; i++) {
        let offset = SAMPLE_OFFSETS[i];
        let d = rectangle_sdf(relative_position + offset, half_screen_size, corner_radius);
        total += step(0.0, -d);
    }
    let alpha = total / 16.0;

    return vec4<f32>(instance.color.rgb, instance.color.a * alpha);
}

const SAMPLE_OFFSETS: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>( 0.000,  0.000),
    vec2<f32>( 0.375,  0.924),
    vec2<f32>( 0.924,  0.375),
    vec2<f32>( 0.924, -0.375),
    vec2<f32>( 0.375, -0.924),
    vec2<f32>(-0.375, -0.924),
    vec2<f32>(-0.924, -0.375),
    vec2<f32>(-0.924,  0.375),
    vec2<f32>(-0.375,  0.924),
    vec2<f32>( 0.476,  0.476),
    vec2<f32>( 0.476, -0.476),
    vec2<f32>(-0.476, -0.476),
    vec2<f32>(-0.476,  0.476),
    vec2<f32>( 0.000,  0.500),
    vec2<f32>( 0.500,  0.000),
    vec2<f32>( 0.000, -0.500)
);


fn draw_sprite(
    instance: InstanceData,
    texture_coordinates: vec2<f32>,
) -> vec4<f32> {
    var color: vec4<f32>;

    if instance.linear_filtering == 0u {
        color = textureSample(textures[instance.texture_index], nearest_sampler, texture_coordinates);
    } else {
        color = textureSample(textures[instance.texture_index], linear_sampler, texture_coordinates);
    }

    return color * instance.color;
}

fn draw_text(
    instance: InstanceData,
    texture_coordinates: vec2<f32>,
) -> vec4<f32> {
    let coverage = textureSample(font_atlas, nearest_sampler, texture_coordinates).r;
    return vec4<f32>(instance.color.rgb, coverage * instance.color.a);
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
    return length(pixel_to_shrunk_corner) - corner_radius;
}
