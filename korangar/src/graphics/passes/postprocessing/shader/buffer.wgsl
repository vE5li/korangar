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

struct DebugUniforms {
    show_picker_buffer: u32,
    show_directional_shadow_map: u32,
    show_point_shadow_map: u32,
    show_light_culling_count_buffer: u32,
    show_font_atlas: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) fragment_position: vec2<f32>,
}

const TILE_SIZE: u32 = 16;

@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var<uniform> debug_uniforms: DebugUniforms;
@group(1) @binding(1) var picker_texture: texture_2d<u32>;
@group(1) @binding(2) var directional_shadow_map: texture_depth_2d;
@group(1) @binding(3) var light_count_texture: texture_2d<u32>;
@group(1) @binding(4) var point_shadow_maps: texture_depth_cube_array;
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

    var output_color: vec4<f32> = vec4<f32>(0.0);

    if (debug_uniforms.show_picker_buffer != 0u) {
        let picker = textureLoad(picker_texture, pixel_coord, 0).rg;
        let red = f32(picker.r & 0xfu) / 100.0;
        let green = f32((picker.r >> 8u) & 0xfu) / 100.0;
        let blue = f32((picker.r >> 16u) & 0xfu) / 100.0;
        output_color += vec4<f32>(red, green, blue, 1.0);
    }

    if (debug_uniforms.show_directional_shadow_map != 0u) {
        var sample_position = clip_to_uv(fragment_position);
        sample_position.y = 1.0 - sample_position.y;
        let depth = textureSample(directional_shadow_map, nearest_sampler, sample_position);
        output_color += vec4<f32>(depth, depth, depth, 1.0);
    }

    if (debug_uniforms.show_point_shadow_map != 0u) {
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

        let depth = textureSample(point_shadow_maps, nearest_sampler, sample_dir, debug_uniforms.show_point_shadow_map - 1u);
        output_color += vec4<f32>(depth, depth, depth, 1.0);
    }

    if (debug_uniforms.show_light_culling_count_buffer != 0u) {
        let tile_coord = pixel_coord / i32(TILE_SIZE);
        let count = textureLoad(light_count_texture, tile_coord, 0).r;

        var color = vec4<f32>(0.0);

        if count != 0 {
            if (count <= 7) {
                let incidence = f32(count) / 7.0;
                color = vec4<f32>(0.0, incidence, 1.0 - incidence, 0.25);
            } else if (count <= 13) {
                let incidence = (f32(count) - 7.0) / 6.0;
                color = vec4<f32>(incidence, 1.0, 0.0, 0.25);
            } else if (count <= 20) {
                let incidence = (f32(count) - 13.0) / 7.0;
                color = vec4<f32>(1.0, 1.0 - incidence, 0.0, 0.25);
            } else {
                color = vec4<f32>(1.0, 0.0, 0.0, 0.25);
            }
        }

        output_color += color;
    }

    if (debug_uniforms.show_font_atlas != 0u) {
        let color = textureSample(font_atlas, nearest_sampler, clip_to_uv(fragment_position));
        output_color += vec4<f32>(color.r, color.r, color.r, 1.0);
    }

    return output_color;
}

fn linearize(raw_value: f32, z_near: f32, z_far: f32) -> f32 {
    return (2.0 * z_near) / (z_far + z_near - raw_value * (z_far - z_near));
}

fn clip_to_uv(clip_space_position: vec2<f32>) -> vec2<f32> {
    return vec2<f32>((clip_space_position.x + 1.0) * 0.5, (1.0 - clip_space_position.y) * 0.5);
}
