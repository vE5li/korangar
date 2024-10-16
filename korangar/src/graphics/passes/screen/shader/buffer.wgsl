struct DebugUniforms {
    show_diffuse_buffer: u32,
    show_normal_buffer: u32,
    show_water_buffer: u32,
    show_depth_buffer: u32,
    show_picker_buffer: u32,
    show_shadow_buffer: u32,
    show_font_atlas: u32,
    show_point_shadow: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
}

@group(0) @binding(4) var<uniform> debug_uniforms: DebugUniforms;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var diffuse_buffer: texture_multisampled_2d<f32>;
@group(1) @binding(1) var normal_buffer: texture_multisampled_2d<f32>;
@group(1) @binding(2) var water_buffer: texture_multisampled_2d<f32>;
@group(1) @binding(3) var depth_buffer: texture_depth_multisampled_2d;
@group(1) @binding(4) var shadow_texture: texture_depth_2d;
@group(1) @binding(5) var point_shadow_maps: binding_array<texture_depth_cube>;
@group(1) @binding(7) var picker_texture: texture_2d<u32>;
@group(2) @binding(0) var font_atlas: texture_2d<f32>;

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

    if (debug_uniforms.show_diffuse_buffer != 0u) {
        let diffuse = textureLoad(diffuse_buffer, pixel_coord, 0).rgb;
        output_color += diffuse;
    }

    if (debug_uniforms.show_normal_buffer != 0u) {
        let normal = textureLoad(normal_buffer, pixel_coord, 0).rgb;
        output_color += normal;
    }

    if (debug_uniforms.show_water_buffer != 0u) {
        let water = textureLoad(water_buffer, pixel_coord, 0);
        output_color += vec3<f32>(0.0, 0.0, water.r);
    }

    if (debug_uniforms.show_depth_buffer != 0u) {
        let depth = 1.0 - textureLoad(depth_buffer, pixel_coord, 0);
        output_color += linearize(depth, 1.0, 2000.0);
    }

    if (debug_uniforms.show_picker_buffer != 0u) {
        let picker = textureLoad(picker_texture, pixel_coord, 0).rg;
        let red = f32(picker.r & 0xfu) / 100.0;
        let green = f32((picker.r >> 8u) & 0xfu) / 100.0;
        let blue = f32((picker.r >> 16u) & 0xfu) / 100.0;
        output_color += vec3<f32>(red, green, blue);
    }

    if (debug_uniforms.show_shadow_buffer != 0u) {
        var sample_position = clip_to_uv(fragment_position);
        sample_position.y = 1.0 - sample_position.y;
        let depth = textureSample(shadow_texture, nearest_sampler, sample_position);
        output_color += vec3<f32>(depth);
    }

    if (debug_uniforms.show_font_atlas != 0u) {
        let color = textureSample(font_atlas, nearest_sampler, clip_to_uv(fragment_position));
        output_color += vec3<f32>(color.r);
    }


    if (debug_uniforms.show_point_shadow != 0u) {
        // +--------+--------+--------+
        // |   +Y   |   +Z   |   +X   |
        // +--------+--------+--------+
        // |   -X   |   -Z   |   -Y   |
        // +--------+--------+--------+

        // Calculate the grid cell (0-5) based on the fragment position.
        let grid_x = floor(fragment_position.x * 1.5 + 1.5);
        let grid_y = floor(fragment_position.y + 1.0);
        let grid_index = i32(grid_x + grid_y * 3.0);

        // Calculate UV coordinates within the cell.
        let cell_uv = vec2<f32>(
            fract(fragment_position.x * 1.5 + 1.5),
            fract(fragment_position.y + 1.0)
        );

        // Map the 2D cell UV to a direction vector for the cube face.
        var sample_dir: vec3<f32>;
        switch (grid_index) {
            case 0: { sample_dir = vec3<f32>(1.0, -cell_uv.y * 2.0 + 1.0, -cell_uv.x * 2.0 + 1.0); }
            case 1: { sample_dir = vec3<f32>(-1.0, -cell_uv.y * 2.0 + 1.0, cell_uv.x * 2.0 - 1.0); }
            case 2: { sample_dir = vec3<f32>(cell_uv.x * 2.0 - 1.0, 1.0, cell_uv.y * 2.0 - 1.0); }
            case 3: { sample_dir = vec3<f32>(cell_uv.x * 2.0 - 1.0, -1.0, -cell_uv.y * 2.0 + 1.0); }
            case 4: { sample_dir = vec3<f32>(cell_uv.x * 2.0 - 1.0, -cell_uv.y * 2.0 + 1.0, 1.0); }
            default: { sample_dir = vec3<f32>(-cell_uv.x * 2.0 + 1.0, -cell_uv.y * 2.0 + 1.0, -1.0); }
        }

        let depth = textureSample(point_shadow_maps[debug_uniforms.show_point_shadow - 1u], nearest_sampler, sample_dir);
        output_color += vec3<f32>(depth, depth, depth);
    }

    return vec4<f32>(output_color, 1.0);
}

fn linearize(raw_value: f32, z_near: f32, z_far: f32) -> f32 {
    return (2.0 * z_near) / (z_far + z_near - raw_value * (z_far - z_near));
}

fn clip_to_uv(clip_space_position: vec2<f32>) -> vec2<f32> {
    return vec2<f32>((clip_space_position.x + 1.0) * 0.5, (1.0 - clip_space_position.y) * 0.5);
}
