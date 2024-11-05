///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
// Copyright (c) 2018, Intel Corporation
//
// Licensed under the Apache License, Version 2.0 ( the "License" );
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

const DEFERRED_APPLY_NUM_THREADS: u32 = 32;

@group(0) @binding(1) var<storage, read_write> control_buffer: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> deferred_blend_item_list_heads: array<atomic<u32>>;
@group(0) @binding(5) var<storage, read_write> deferred_blend_item_list: array<vec2<u32>>;
@group(0) @binding(6) var<storage, read_write> deferred_blend_location_list: array<u32>;
@group(1) @binding(0) var output_texture: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(4, DEFERRED_APPLY_NUM_THREADS, 1)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let num_candidates = atomicLoad(&control_buffer[0]);
    let deferred_blend_location_list_length = arrayLength(&deferred_blend_location_list);

    let current_candidate = global_id.y;
    let current_quad_offset_xy = local_id.x;

    if (current_candidate >= num_candidates || current_candidate >= deferred_blend_location_list_length) {
        return;
    }

    let output_texture_size = vec2<f32>(textureDimensions(output_texture));
    let deferred_blend_item_list_heads_width = ((u32(output_texture_size.x) + 1u) / 2u);

    let pixel_id = deferred_blend_location_list[current_candidate];
    let quad_pos = vec2<u32>((pixel_id >> 16u), pixel_id & 0xFFFFu);
    let pixel_pos = quad_pos * 2u + qe_offsets(current_quad_offset_xy);
    let quad_index = quad_pos.y * deferred_blend_item_list_heads_width + quad_pos.x;

    var counter_index_with_header = atomicLoad(&deferred_blend_item_list_heads[quad_index]);

    var out_color = vec4<f32>(0.0);

    // Do the loop to prevent bad data hanging the GPU.
    let MAX_LOOPS = 32u;

    for (var i = 0u; counter_index_with_header != 0xFFFFFFFFu && i < MAX_LOOPS; i++) {
        // Decode item-specific info:
        // - 2 bits for 2x2 quad location
        // - 1 bit for isComplexShape flag
        // - 29 bits left for address (counter_index)
        let offset_xy = (counter_index_with_header >> 30u) & 0x03u;
        let is_complex_shape = ((counter_index_with_header >> 29u) & 0x01u) != 0u;

        let index = counter_index_with_header & ((1u << 29u) - 1u);
        let value = deferred_blend_item_list[index];
        counter_index_with_header = value.x;

        if (offset_xy == current_quad_offset_xy) {
            let color = unpack_r11g11b10_u32(value.y);
            let weight = 0.8 + f32(is_complex_shape);
            out_color += vec4<f32>(color * weight, weight);
        }
    }

    if (out_color.a == 0.0) {
        return;
    }

    let final_color = out_color.rgb / out_color.a;

    textureStore(output_texture, pixel_pos, vec4<f32>(final_color, 1.0));
}

// Truth table:
// index | x | y
//   0   | 0 | 0
//   1   | 1 | 0
//   2   | 0 | 1
//   3   | 1 | 1
fn qe_offsets(index: u32) -> vec2<u32> {
    let x = index & 1u;
    let y = (index >> 1u) & 1u;
    return vec2<u32>(x, y);
}

fn unpack_r11g11b10_u32(packed: u32) -> vec3<f32> {
    let r = f32(packed & 0x7FFu) / 2047.0;
    let g = f32((packed >> 11u) & 0x7FFu) / 2047.0;
    let b = f32((packed >> 22u) & 0x3FFu) / 1023.0;
    return vec3<f32>(r, g, b);
}
