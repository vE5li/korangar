struct Constants {
    show_diffuse_buffer: u32,
    show_normal_buffer: u32,
    show_water_buffer: u32,
    show_depth_buffer: u32,
    show_picker_texture: u32,
    show_shadow_texture: u32,
    show_font_atlas: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
}


@group(0) @binding(0) var diffuse_buffer: texture_multisampled_2d<f32>;
@group(0) @binding(1) var normal_buffer: texture_multisampled_2d<f32>;
@group(0) @binding(2) var water_buffer: texture_multisampled_2d<f32>;
@group(0) @binding(3) var depth_buffer: texture_depth_multisampled_2d;
@group(0) @binding(4) var picker_texture: texture_2d<u32>;
@group(0) @binding(5) var shadow_texture: texture_depth_2d;
@group(0) @binding(6) var font_atlas: texture_2d<f32>;
@group(0) @binding(7) var nearest_sampler: sampler;

var<push_constant> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    let vertex = vec2<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0));

    var output: VertexOutput;
    output.position = vec4<f32>(vertex, 0.0, 1.0);
    output.fragment_position = vertex;
    return output;
}

@fragment
fn fs_main(
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
) -> @location(0) vec4<f32> {
    let pixel_coord = vec2<i32>(position.xy);

    var output_color: vec3<f32> = vec3<f32>(0.0);

    if (constants.show_diffuse_buffer != 0u) {
        let diffuse = textureLoad(diffuse_buffer, pixel_coord, 0).rgb;
        output_color += diffuse;
    }

    if (constants.show_normal_buffer != 0u) {
        let normal = textureLoad(normal_buffer, pixel_coord, 0).rgb;
        output_color += normal;
    }

    if (constants.show_water_buffer != 0u) {
        let water = textureLoad(water_buffer, pixel_coord, 0);
        output_color += vec3<f32>(0.0, 0.0, water.r);
    }

    if (constants.show_depth_buffer != 0u) {
        let depth = 1.0 - textureLoad(depth_buffer, pixel_coord, 0);
        output_color += linearize(depth, 1.0, 2000.0);
    }

    if (constants.show_picker_texture != 0u) {
        let picker = textureLoad(picker_texture, pixel_coord, 0).r;
        let red = f32(picker & 0xfu) / 100.0;
        let green = f32((picker >> 8u) & 0xfu) / 100.0;
        let blue = f32((picker >> 16u) & 0xfu) / 100.0;
        output_color += vec3<f32>(red, green, blue);
    }

    if (constants.show_shadow_texture != 0u) {
        var sample_position = clip_to_uv(fragment_position);
        sample_position.y = 1.0 - sample_position.y;
        let depth = textureSample(shadow_texture, nearest_sampler, sample_position);
        output_color += vec3<f32>(depth);
    }

    if (constants.show_font_atlas != 0u) {
        let color = textureSample(font_atlas, nearest_sampler, clip_to_uv(fragment_position));
        output_color += vec3<f32>(color.r);
    }

    return vec4<f32>(output_color, 1.0);
}

fn linearize(raw_value: f32, z_near: f32, z_far: f32) -> f32 {
    return (2.0 * z_near) / (z_far + z_near - raw_value * (z_far - z_near));
}

fn clip_to_uv(clip_space_position: vec2<f32>) -> vec2<f32> {
    return vec2<f32>((clip_space_position.x + 1.0) * 0.5, (1.0 - clip_space_position.y) * 0.5);
}
