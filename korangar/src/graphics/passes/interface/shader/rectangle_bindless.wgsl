struct InstanceData {
    color: vec4<f32>,
    corner_radius: vec4<f32>,
    screen_clip: vec4<f32>,
    screen_position: vec2<f32>,
    screen_size: vec2<f32>,
    texture_position: vec2<f32>,
    texture_size: vec2<f32>,
    aspect_ratio: f32,
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
    output.fragment_position = vertex.zw;
    output.texture_coordinates = instance.texture_position + vertex.zw * instance.texture_size;
    output.instance_index = instance_index;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let instance = instance_data[input.instance_index];

    switch (instance.rectangle_type) {
        case 0u: {
            return draw_solid(instance, input.position, input.fragment_position);
        }
        case 1u: {
            return draw_sprite(instance, input.position, input.texture_coordinates, instance.texture_index, instance.linear_filtering);
        }
        default: {
            return draw_text(instance, input.position, input.texture_coordinates);
        }
    }
}

fn draw_solid(
    instance: InstanceData,
    position: vec4<f32>,
    fragment_position: vec2<f32>
) -> vec4<f32> {
    var coords = fragment_position * instance.screen_size;
    var screen_size = instance.screen_size;
    let corner_radius = instance.corner_radius * 0.5;

    coords.x /= instance.aspect_ratio;
    screen_size.x /= instance.aspect_ratio;

    // top-left
    if (length(coords - corner_radius.x) > corner_radius.x && coords.x < corner_radius.x &&
        coords.y < corner_radius.x) {
        discard;
    }

    // top-right
    if (length(coords - vec2<f32>(screen_size.x - corner_radius.y, corner_radius.y)) > corner_radius.y &&
        screen_size.x - coords.x < corner_radius.y && coords.y < corner_radius.y) {
        discard;
    }

    // bottom-right
    if (length(coords - screen_size + corner_radius.z) > corner_radius.z &&
        screen_size.x - coords.x < corner_radius.z && screen_size.y - coords.y < corner_radius.z) {
        discard;
    }

    // bottom-left
    if (length(coords - vec2<f32>(corner_radius.w, screen_size.y - corner_radius.w)) > corner_radius.w &&
        coords.x < corner_radius.w && screen_size.y - coords.y < corner_radius.w) {
        discard;
    }

    if (position.x < instance.screen_clip.x || position.y < instance.screen_clip.y ||
        position.x > instance.screen_clip.z || position.y > instance.screen_clip.w) {
        discard;
    }

    return instance.color;
}

fn draw_sprite(
    instance: InstanceData,
    position: vec4<f32>,
    texture_coordinates: vec2<f32>,
    texture_index: i32,
    linear_filtering: u32
) -> vec4<f32> {
    var color: vec4<f32>;

    if linear_filtering == 0u {
        color = textureSample(textures[texture_index], nearest_sampler, texture_coordinates);
    } else {
        color = textureSample(textures[texture_index], linear_sampler, texture_coordinates);
    }

    if (position.x < instance.screen_clip.x || position.y < instance.screen_clip.y ||
        position.x > instance.screen_clip.z || position.y > instance.screen_clip.w) {
        discard;
    }

    return color * instance.color;
}

fn draw_text(
    instance: InstanceData,
    position: vec4<f32>,
    texture_coordinates: vec2<f32>,
) -> vec4<f32> {
    let coverage = textureSample(font_atlas, linear_sampler, texture_coordinates).r;

    if (position.x < instance.screen_clip.x || position.y < instance.screen_clip.y ||
        position.x > instance.screen_clip.z || position.y > instance.screen_clip.w) {
        discard;
    }

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
