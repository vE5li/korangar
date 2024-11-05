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

const PROCESS_CANDIDATES_NUM_THREADS: u32 = 128;
const BLEND_ITEM_SLM_SIZE: u32 = 768;
const SIMPLE_SHAPE_BLURINESS_AMOUNT: f32 = 0.10;
/// Longest line search distance; must be even number; for high perf low quality start from ~32 - the bigger the number,
/// the nicer the gradients but more costly. Max supported is 128!
const MAX_LINE_LENGTH: f32 = 86.0;
const SYMETRY_CORRECTION_OFFSET: f32 = 0.22;
const DAMPENING_EFFECT: f32 = 0.15;

@group(0) @binding(0) var edges: texture_storage_2d<r8uint, read_write>;
@group(0) @binding(1) var<storage, read_write> control_buffer: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> shape_candidates: array<u32>;
@group(0) @binding(4) var<storage, read_write> deferred_blend_item_list_heads: array<atomic<u32>>;
@group(0) @binding(5) var<storage, read_write> deferred_blend_item_list: array<vec2<u32>>;
@group(0) @binding(6) var<storage, read_write> deferred_blend_location_list: array<u32>;
@group(1) @binding(0) var input_texture: texture_2d<f32>;

var<workgroup> blend_item_count: atomic<u32>;
var<workgroup> blend_items: array<vec2<u32>, BLEND_ITEM_SLM_SIZE>;
var<workgroup> deferred_blend_item_list_heads_width: u32;
var<workgroup> deferred_blend_item_list_length: u32;
var<workgroup> deferred_blend_location_list_length: u32;

@compute @workgroup_size(PROCESS_CANDIDATES_NUM_THREADS, 1, 1)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    if (local_id.x == 0u) {
        atomicStore(&blend_item_count, 0u);

        let input_texture_size = vec2<f32>(textureDimensions(input_texture));
        deferred_blend_item_list_heads_width = ((u32(input_texture_size.x) + 1u) / 2u);
        deferred_blend_item_list_length = arrayLength(&deferred_blend_item_list);
        deferred_blend_location_list_length = arrayLength(&deferred_blend_location_list);
    }

    workgroupBarrier();

    let num_candidates = atomicLoad(&control_buffer[0]);

    if (global_id.x < num_candidates) {
        let pixel_id = shape_candidates[global_id.x];
        let pixel_pos = vec2<u32>((pixel_id >> 16u), pixel_id & 0xFFFFu);

        let edges_center_packed = load_edge(pixel_pos, vec2<i32>(0, 0));
        let edges        = unpack_edges_float(edges_center_packed);
        let edges_left   = unpack_edges_float(load_edge(pixel_pos, vec2<i32>(-1, 0)));
        let edges_right  = unpack_edges_float(load_edge(pixel_pos, vec2<i32>(1, 0)));
        let edges_bottom = unpack_edges_float(load_edge(pixel_pos, vec2<i32>(0, 1)));
        let edges_top    = unpack_edges_float(load_edge(pixel_pos, vec2<i32>(0, -1)));

        // Simple shapes
        {
            let blend_val = compute_simple_shape_blend_values(edges, edges_left, edges_right, edges_top, edges_bottom, true);

            let four_weight_sum = dot(blend_val, vec4<f32>(1.0));
            let center_weight = 1.0 - four_weight_sum;

            var out_color = load_source_color(pixel_pos, vec2<i32>(0, 0)).rgb * center_weight;

            // from left
            if (blend_val.x > 0.0) {
                let pixel_l = load_source_color(pixel_pos, vec2<i32>(-1, 0)).rgb;
                out_color += blend_val.x * pixel_l;
            }
            // from above
            if (blend_val.y > 0.0) {
                let pixel_t = load_source_color(pixel_pos, vec2<i32>(0, -1)).rgb;
                out_color += blend_val.y * pixel_t;
            }
            // from right
            if (blend_val.z > 0.0) {
                let pixel_r = load_source_color(pixel_pos, vec2<i32>(1, 0)).rgb;
                out_color += blend_val.z * pixel_r;
            }
            // from below
            if (blend_val.w > 0.0) {
                let pixel_b = load_source_color(pixel_pos, vec2<i32>(0, 1)).rgb;
                out_color += blend_val.w * pixel_b;
            }

            store_color_sample(pixel_pos, out_color, false);
        }

        // Complex shapes - detect
        {
            var inverted_z_score: f32 = 0.0;
            var normal_z_score: f32 = 0.0;
            var max_score: f32 = 0.0;
            var horizontal = true;
            var inverted_z = false;

            // Horizontal
            {
                let edges_m1p0 = edges_left;
                let edges_p1p0 = edges_right;
                let edges_p2p0 = unpack_edges_float(load_edge(pixel_pos, vec2<i32>(2, 0)));

                detect_zs_horizontal(edges, edges_m1p0, edges_p1p0, edges_p2p0, &inverted_z_score, &normal_z_score);
                max_score = max(inverted_z_score, normal_z_score);

                if (max_score > 0.0) {
                    inverted_z = inverted_z_score > normal_z_score;
                }
            }

            // Vertical
            {
                // Reuse the same code for vertical (used for horizontal above), but rotate
                // input data 90 degrees counter-clockwise, so that:
                //
                // left     becomes     bottom
                // top      becomes     left
                // right    becomes     top
                // bottom   becomes     right

                // We also have to rotate edges, thus .argb
                let edges_m1p0 = edges_bottom;
                let edges_p1p0 = edges_top;
                let edges_p2p0 = unpack_edges_float(load_edge(pixel_pos, vec2<i32>(0, -2)));

                detect_zs_horizontal(edges.argb, edges_m1p0.argb, edges_p1p0.argb, edges_p2p0.argb, &inverted_z_score, &normal_z_score);
                let vert_score = max(inverted_z_score, normal_z_score);

                if (vert_score > max_score) {
                    max_score = vert_score;
                    horizontal = false;
                    inverted_z = inverted_z_score > normal_z_score;
                }
            }

            if (max_score > 0.0) {
                let shape_quality_score = floor(clamp(4.0 - max_score, 0.0, 3.0));
                let step_right = select(vec2<f32>(0.0, -1.0), vec2<f32>(1.0, 0.0), horizontal);

                var line_length_left: f32 = 0.0;
                var line_length_right: f32 = 0.0;

                find_z_line_lengths(&line_length_left, &line_length_right, pixel_pos, horizontal, inverted_z, step_right);

                line_length_left -= shape_quality_score;
                line_length_right -= shape_quality_score;

                if ((line_length_left + line_length_right) >= 5.0) {
                    // Try adding to SLM but fall back to in-place processing if full (which only really happens in synthetic test cases)
                    if (!collect_blend_zs(pixel_pos, horizontal, inverted_z, shape_quality_score, line_length_left, line_length_right, step_right)) {
                        blend_zs(pixel_pos, horizontal, inverted_z, shape_quality_score, line_length_left, line_length_right, step_right);
                    }
                }
            }
        }
    }

    workgroupBarrier();

    let total_item_count = min(BLEND_ITEM_SLM_SIZE, atomicLoad(&blend_item_count));

    // Spread items into waves
    let loops = (total_item_count + (PROCESS_CANDIDATES_NUM_THREADS - 1u) - local_id.x) / PROCESS_CANDIDATES_NUM_THREADS;

    for (var loop_index = 0u; loop_index < loops; loop_index++) {
        let index = loop_index * PROCESS_CANDIDATES_NUM_THREADS + local_id.x;
        let item_val = blend_items[index];

        let starting_pos = vec2<u32>((item_val.x >> 16u), item_val.x & 0xFFFFu);

        let item_horizontal = bool((item_val.y >> 31u) & 1u);
        let item_inverted_z = bool((item_val.y >> 30u) & 1u);
        let item_step_index = f32((item_val.y >> 20u) & 0x3FFu) - 256.0;
        let item_src_offset = f32((item_val.y >> 10u) & 0x3FFu) - 256.0;
        let item_lerp_k = f32(item_val.y & 0x3FFu) / 1023.0;

        let item_step_right = select(vec2<f32>(0.0, -1.0), vec2<f32>(1.0, 0.0), item_horizontal);
        var item_blend_dir = select(vec2<f32>(-1.0, 0.0), vec2<f32>(0.0, -1.0), item_horizontal);
        if (item_inverted_z) {
            item_blend_dir = -item_blend_dir;
        }

        let item_pixel_pos = vec2<u32>(vec2<f32>(starting_pos) + item_step_right * item_step_index);

        let color_center = load_source_color(item_pixel_pos, vec2<i32>(0, 0)).rgb;
        let color_from = load_source_color(
            vec2<u32>(vec2<f32>(item_pixel_pos) + item_blend_dir * vec2<f32>(item_src_offset, item_src_offset)),
            vec2<i32>(0, 0)
        ).rgb;

        let output_color = mix(color_center, color_from, item_lerp_k);
        store_color_sample(item_pixel_pos, output_color, true);
    }
}

fn unpack_edges_float(value: u32) -> vec4<f32> {
    return vec4<f32>(
        f32((value & 0x01u) != 0u),
        f32((value & 0x02u) != 0u),
        f32((value & 0x04u) != 0u),
        f32((value & 0x08u) != 0u)
    );
}

fn load_edge(pixel_pos: vec2<u32>, offset: vec2<i32>) -> u32 {
    let a = (pixel_pos.x + u32(offset.x)) % 2u;
    let edge = textureLoad(edges,
        vec2<i32>(i32((pixel_pos.x + u32(offset.x))/2u),
        i32(pixel_pos.y) + offset.y)
    ).x;
    return (edge >> (a * 4u)) & 0xFu;
}


fn load_source_color(pixel_pos: vec2<u32>, offset: vec2<i32>) -> vec3<f32> {
    return textureLoad(input_texture, pixel_pos + vec2<u32>(offset), 0).rgb;
}

fn store_color_sample(
    pixel_pos: vec2<u32>,
    color: vec3<f32>,
    is_complex_shape: bool,
) {
    let counter_index = atomicAdd(&control_buffer[3], 1u);

    if counter_index >= deferred_blend_item_list_length {
        return;
    }

    // Quad coordinates
    let quad_pos = pixel_pos >> vec2<u32>(1u);
    let quad_index = quad_pos.y * deferred_blend_item_list_heads_width + quad_pos.x;

    // 2x2 inter-quad coordinates
    let offset_xy = (pixel_pos.y % 2u) * 2u + (pixel_pos.x % 2u);

    // Encode item-specific info:
    // - 2 bits for 2x2 quad location
    // - 1 bit for isComplexShape flag
    // - 29 bits left for address (counter_index)
    let header = (offset_xy << 30u) | (u32(is_complex_shape) << 29u);
    let counter_index_with_header = counter_index | header;
    var original_index = atomicExchange(&deferred_blend_item_list_heads[quad_index], counter_index_with_header);

    deferred_blend_item_list[counter_index] = vec2<u32>(original_index, pack_r11g11b10_u32(color));

    // First one added?
    if (original_index == 0xFFFFFFFFu) {
        // Make a list of all edge pixels - these cover all potential pixels where AA is applied
        let edge_list_counter = atomicAdd(&control_buffer[2], 1u);
        if edge_list_counter < deferred_blend_location_list_length {
                deferred_blend_location_list[edge_list_counter] = (quad_pos.x << 16u) | quad_pos.y;
        }
    }
}

fn pack_r11g11b10_u32(rgb: vec3<f32>) -> u32 {
    let r = u32(clamp(rgb.r, 0.0, 1.0) * 2047.0); // 11 bits (0-2047)
    let g = u32(clamp(rgb.g, 0.0, 1.0) * 2047.0); // 11 bits (0-2047)
    let b = u32(clamp(rgb.b, 0.0, 1.0) * 1023.0); // 10 bits (0-1023)
    return (r & 0x7FFu) | ((g & 0x7FFu) << 11u) | ((b & 0x3FFu) << 22u);
}

fn compute_simple_shape_blend_values(
    edges: vec4<f32>,
    edges_left: vec4<f32>,
    edges_right: vec4<f32>,
    edges_top: vec4<f32>,
    edges_bottom: vec4<f32>,
    dont_test_shape_validity: bool,
) -> vec4<f32> {
    var from_right = edges.r;
    var from_below = edges.g;
    var from_left = edges.b;
    var from_above = edges.a;

    var blur_coeff = SIMPLE_SHAPE_BLURINESS_AMOUNT;
    let number_of_edges = dot(edges, vec4<f32>(1.0));

    let number_of_edges_all_around = dot(
        edges_left.bga + edges_right.rga + edges_top.rba + edges_bottom.rgb,
        vec3<f32>(1.0)
    );

    // Skip if already tested for before calling this function
    if (!dont_test_shape_validity) {
        // No blur for straight edge
        if (number_of_edges == 1.0) {
            blur_coeff = 0.0;
        }
        // L-like step shape (only blur if it's a corner, not if it's two parallel edges)
        if (number_of_edges == 2.0) {
            blur_coeff *= (1.0 - from_below * from_above) * (1.0 - from_right * from_left);
        }
    }

    // L-like step shape
    if (number_of_edges == 2.0) {
        blur_coeff *= 0.75;
        let k = 0.9;

        from_right += k * (edges.g * edges_top.r * (1.0 - edges_left.g) + edges.a * edges_bottom.r * (1.0 - edges_left.a));
        from_below += k * (edges.b * edges_right.g * (1.0 - edges_top.b) + edges.r * edges_left.g * (1.0 - edges_top.r));
        from_left += k * (edges.a * edges_bottom.b * (1.0 - edges_right.a) + edges.g * edges_top.b * (1.0 - edges_right.g));
        from_above += k * (edges.r * edges_left.a * (1.0 - edges_bottom.r) + edges.b * edges_right.a * (1.0 - edges_bottom.b));
    }

    // Dampen blurring effect when lots of neighbouring edges
    blur_coeff *= saturate(1.30 - number_of_edges_all_around / 10.0);

    return vec4<f32>(from_left, from_above, from_right, from_below) * blur_coeff;
}

fn detect_zs_horizontal(
    edges: vec4<f32>,
    edges_m1p0: vec4<f32>,
    edges_p1p0: vec4<f32>,
    edges_p2p0: vec4<f32>,
    inverted_z_score: ptr<function, f32>,
    normal_z_score: ptr<function, f32>
) {
    // Inverted Z case:
    //   __
    //  X|
    // --
    *inverted_z_score = edges.r * edges.g * edges_p1p0.a;
    *inverted_z_score *= 2.0 + (edges_m1p0.g + edges_p2p0.a) - (edges.a + edges_p1p0.g)
        - 0.7 * (edges_p2p0.g + edges_m1p0.a + edges.b + edges_p1p0.r);

    // Normal Z case:
    // __
    //  X|
    //   --
    *normal_z_score = edges.r * edges.a * edges_p1p0.g;
    *normal_z_score *= 2.0 + (edges_m1p0.a + edges_p2p0.g) - (edges.g + edges_p1p0.a)
        - 0.7 * (edges_p2p0.a + edges_m1p0.g + edges.b + edges_p1p0.r);
}

fn find_z_line_lengths(
    line_length_left: ptr<function, f32>,
    line_length_right: ptr<function, f32>,
    screen_pos: vec2<u32>,
    horizontal: bool,
    inverted_z_shape: bool,
    step_right: vec2<f32>,
) {
    // Horizontal (vertical is the same, just rotated 90- counter-clockwise)
    // Inverted Z case:              // Normal Z case:
    //   __                          // __
    //  X|                           //  X|
    // --                            //   --
    let mask_trace_left = select(0x04u, 0x08u, horizontal);
    let mask_trace_right = select(0x01u, 0x02u, horizontal);

    let mask_left = select(mask_trace_left, mask_trace_right, inverted_z_shape);
    let mask_right = select(mask_trace_right, mask_trace_left, inverted_z_shape);
    let bits_continue_left = mask_left;
    let bits_continue_right = mask_right;

    var continue_left = true;
    var continue_right = true;
    *line_length_left = 1.0;
    *line_length_right = 1.0;

    loop {
        let edge_left  = load_edge(screen_pos - vec2<u32>(step_right * *line_length_left), vec2<i32>(0));
        let edge_right = load_edge(screen_pos + vec2<u32>(step_right * (*line_length_right + 1.0)), vec2<i32>(0));

        // Stop on encountering 'stopping' edge (as defined by masks)
        continue_left  = continue_left  && ((edge_left  & mask_left)  == bits_continue_left);
        continue_right = continue_right && ((edge_right & mask_right) == bits_continue_right);

        *line_length_left  += f32(continue_left);
        *line_length_right += f32(continue_right);

        var max_lr = max(*line_length_right, *line_length_left);

        // Both stopped? Cause the search end by setting max_lr to MAX_LINE_LENGTH.
        if (!continue_left && !continue_right) {
            max_lr = MAX_LINE_LENGTH;
        }

        if (max_lr >= min(MAX_LINE_LENGTH, 1.25 * min(*line_length_right, *line_length_left) - 0.25)) {
            break;
        }
    }
}

fn collect_blend_zs(
    screen_pos: vec2<u32>,
    horizontal: bool,
    inverted_z_shape: bool,
    shape_quality_score: f32,
    line_length_left: f32,
    line_length_right: f32,
    step_right: vec2<f32>,
) -> bool {
    let left_odd  = SYMETRY_CORRECTION_OFFSET * f32(u32(line_length_left) % 2u);
    let right_odd = SYMETRY_CORRECTION_OFFSET * f32(u32(line_length_right) % 2u);

    let dampen_effect = saturate(f32(line_length_left + line_length_right - shape_quality_score) * DAMPENING_EFFECT);

    let loop_from = -floor((line_length_left + 1.0) / 2.0) + 1.0;
    let loop_to = floor((line_length_right + 1.0) / 2.0);

    let item_count = u32(loop_to - loop_from + 1.0);
    var item_index = atomicAdd(&blend_item_count, item_count);

    if ((item_index + item_count) > BLEND_ITEM_SLM_SIZE) {
        return false;
    }

    let total_length = (loop_to - loop_from) + 1.0 - left_odd - right_odd;
    let lerp_step = 1.0 / total_length;
    let lerp_from_k = (0.5 - left_odd - loop_from) * lerp_step;

    let item_header = (screen_pos.x << 16u) | screen_pos.y;
    let item_val_static = (u32(horizontal) << 31u) | (u32(inverted_z_shape) << 30u);

    for(var i = loop_from; i <= loop_to; i += 1.0) {
        let second_part = f32(i > 0.0);
        let src_offset = 1.0 - second_part * 2.0;

        let lerp_k = ((lerp_step * i + lerp_from_k) * src_offset + second_part) * dampen_effect;

        let encoded_Item = vec2<u32>(
            item_header,
            item_val_static | (u32(i + 256.0) << 20u) | (u32(src_offset + 256.0) << 10u) | u32(saturate(lerp_k) * 1023.0 + 0.5)
        );
        blend_items[item_index] = encoded_Item;
        item_index += 1u;
    }

    return true;
}

fn blend_zs(
    screen_pos: vec2<u32>,
    horizontal: bool,
    inverted_z_shape: bool,
    shape_quality_score: f32,
    line_length_left: f32,
    line_length_right: f32,
    step_right: vec2<f32>,
) {
    let blend_dir = select(vec2<f32>(-1.0, 0.0), vec2<f32>(0.0, -1.0), horizontal);
    let final_blend_dir = select(blend_dir, -blend_dir, inverted_z_shape);

    let left_odd  = SYMETRY_CORRECTION_OFFSET * f32(u32(line_length_left) % 2u);
    let right_odd = SYMETRY_CORRECTION_OFFSET * f32(u32(line_length_right) % 2u);

    let dampen_effect = saturate(f32(line_length_left + line_length_right - shape_quality_score) * DAMPENING_EFFECT);

    let loop_from = -floor((line_length_left + 1.0) / 2.0) + 1.0;
    let loop_to = floor((line_length_right + 1.0) / 2.0);

    let total_length = (loop_to - loop_from) + 1.0 - left_odd - right_odd;
    let lerp_step = 1.0 / total_length;
    let lerp_from_k = (0.5 - left_odd - loop_from) * lerp_step;

    for(var i = loop_from; i <= loop_to; i += 1.0) {
        let second_part = f32(i > 0.0);
        let src_offset = 1.0 - second_part * 2.0;

        let lerp_k = ((lerp_step * i + lerp_from_k) * src_offset + second_part) * dampen_effect;

        let pixel_pos = vec2<u32>(vec2<f32>(screen_pos) + step_right * i);

        let color_center = load_source_color(pixel_pos, vec2<i32>(0)).rgb;
        let color_from = load_source_color(pixel_pos + vec2<u32>(final_blend_dir * src_offset), vec2<i32>(0)).rgb;

        let output = mix(color_center, color_from, lerp_k);
        store_color_sample(pixel_pos, output, true);
    }
}
