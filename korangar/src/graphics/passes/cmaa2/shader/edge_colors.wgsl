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

const INPUT_KERNEL_SIZE_X: u32 = 16;
const INPUT_KERNEL_SIZE_Y: u32 = 16;
const OUTPUT_KERNEL_SIZE_X = INPUT_KERNEL_SIZE_X - 2;
const OUTPUT_KERNEL_SIZE_Y = INPUT_KERNEL_SIZE_Y - 2;
const LOCAL_CONTRAST_ADAPTATION_AMOUNT: f32 = 0.10;
const EDGE_THRESHOLD: f32 = 0.12;

@group(0) @binding(0) var edges: texture_storage_2d<r8uint, read_write>;
@group(0) @binding(1) var<storage, read_write> control_buffer: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> shape_candidates: array<u32>;
@group(0) @binding(4) var<storage, read_write> deferred_blend_item_list_heads: array<atomic<u32>>;
@group(1) @binding(0) var input_texture: texture_2d<f32>;

var<workgroup> edges_v: array<vec4<f32>, 256>;
var<workgroup> edges_h: array<vec4<f32>, 256>;

var<private> pixel_colors: array<vec3<f32>, 8>;
var<private> neighbourhood: array<mat4x2<f32>, 4>;

@compute @workgroup_size(16, 16, 1)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    // Screen position in the input (expanded) kernel (shifted one 2x2 block up/left)
    let pixel_pos = (workgroup_id.xy * vec2<u32>(OUTPUT_KERNEL_SIZE_X, OUTPUT_KERNEL_SIZE_Y)
                     + local_id.xy - vec2<u32>(1u, 1u)) * vec2<u32>(2u, 2u);

    let row_stride_2x2 = INPUT_KERNEL_SIZE_X;
    let center_addr_2x2 = local_id.x + local_id.y * row_stride_2x2;
    let in_output_kernel = !(local_id.x == (INPUT_KERNEL_SIZE_X - 1u) ||
                            local_id.x == 0u ||
                            local_id.y == (INPUT_KERNEL_SIZE_Y - 1u) ||
                            local_id.y == 0u);

    for (var i = 0; i < 8; i++) {
        pixel_colors[i] = load_source_color(pixel_pos, vec2<i32>(i % 3, i / 3)).rgb;
    }

    for (var i = 0; i < 8; i++) {
        pixel_colors[i] = process_color_for_edge_detect(pixel_colors[i]);
    }

    let qe0 = compute_edge(0, 0, &pixel_colors);
    let qe1 = compute_edge(1, 0, &pixel_colors);
    let qe2 = compute_edge(0, 1, &pixel_colors);
    let qe3 = compute_edge(1, 1, &pixel_colors);

    // Store edges in workgroup shared memory
    edges_v[center_addr_2x2 + row_stride_2x2 * 0] = vec4<f32>(qe0.x, qe1.x, qe2.x, qe3.x);
    edges_h[center_addr_2x2 + row_stride_2x2 * 0] = vec4<f32>(qe0.y, qe1.y, qe2.y, qe3.y);

    workgroupBarrier();

    if (in_output_kernel) {
        let shape_candidates_length = arrayLength(&shape_candidates);

        var out_edges = vec4<u32>(0);

        // top row's bottom edge
        var top_row = edges_h[center_addr_2x2 - row_stride_2x2].zw;
        // left column's right edge
        var left_column = edges_v[center_addr_2x2 - 1].yw;

        let some_non_zero_edges = any(abs(qe0) + abs(qe1) + abs(qe2) + abs(qe3) > vec2<f32>(0.0)) ||
            any(vec4<f32>(top_row.x, top_row.y, left_column.x, left_column.y) > vec4<f32>(0.0));

        if (some_non_zero_edges) {
            // Clear deferred color list heads to empty (if potentially needed - even though some edges might get culled by local contrast adaptation
            // step below, it's still cheaper to just clear it without additional logic).
            let input_texture_size = vec2<f32>(textureDimensions(input_texture));
            let deferred_blend_item_list_heads_width = ((u32(input_texture_size.x) + 1u) / 2u);
            let quad_pos = pixel_pos >> vec2<u32>(1u);
            let quad_index = quad_pos.y * deferred_blend_item_list_heads_width + quad_pos.x;
            atomicStore(&deferred_blend_item_list_heads[quad_index], 0xFFFFFFFFu);

            // Load neighborhood data from workgroup memory
            let n00 = load_quad_hv(center_addr_2x2 - row_stride_2x2 - 1u);
            let n10 = load_quad_hv(center_addr_2x2 - row_stride_2x2);
            let n20 = load_quad_hv(center_addr_2x2 - row_stride_2x2 + 1u);
            let n01 = load_quad_hv(center_addr_2x2 - 1u);
            let n21 = load_quad_hv(center_addr_2x2 + 1u);
            let n02 = load_quad_hv(center_addr_2x2 - 1u + row_stride_2x2);
            let n12 = load_quad_hv(center_addr_2x2 + row_stride_2x2);

            neighbourhood[0][0] = n00[3];
            neighbourhood[0][1] = vec2<f32>(left_column[0], n01[1].y);
            neighbourhood[0][2] = vec2<f32>(left_column[1], n01[3].y);
            neighbourhood[0][3] = n02[1];

            neighbourhood[1][0] = vec2<f32>(n10[2].x, top_row[0]);
            neighbourhood[1][1] = qe0;
            neighbourhood[1][2] = qe2;
            neighbourhood[1][3] = n12[0];

            neighbourhood[2][0] = vec2<f32>(n10[3].x, top_row[1]);
            neighbourhood[2][1] = qe1;
            neighbourhood[2][2] = qe3;
            neighbourhood[2][3] = n12[1];

            neighbourhood[3][0] = n20[2];
            neighbourhood[3][1] = n21[0];
            neighbourhood[3][2] = n21[2];
            neighbourhood[3][3] = vec2<f32>(0.0);

            top_row.x = f32((top_row.x -         compute_local_contrast_h(0, -1, &neighbourhood)) > EDGE_THRESHOLD);
            top_row.y = f32((top_row.y -         compute_local_contrast_h(1, -1, &neighbourhood)) > EDGE_THRESHOLD);
            left_column.x = f32((left_column.x - compute_local_contrast_v(-1, 0, &neighbourhood)) > EDGE_THRESHOLD);
            left_column.y = f32((left_column.y - compute_local_contrast_v(-1, 1, &neighbourhood)) > EDGE_THRESHOLD);

            var ce: mat4x4<f32>;

            ce[0].x = f32((qe0.x - compute_local_contrast_v(0, 0, &neighbourhood)) > EDGE_THRESHOLD);
            ce[0].y = f32((qe0.y - compute_local_contrast_h(0, 0, &neighbourhood)) > EDGE_THRESHOLD);
            ce[1].x = f32((qe1.x - compute_local_contrast_v(1, 0, &neighbourhood)) > EDGE_THRESHOLD);
            ce[1].y = f32((qe1.y - compute_local_contrast_h(1, 0, &neighbourhood)) > EDGE_THRESHOLD);
            ce[2].x = f32((qe2.x - compute_local_contrast_v(0, 1, &neighbourhood)) > EDGE_THRESHOLD);
            ce[2].y = f32((qe2.y - compute_local_contrast_h(0, 1, &neighbourhood)) > EDGE_THRESHOLD);
            ce[3].x = f32((qe3.x - compute_local_contrast_v(1, 1, &neighbourhood)) > EDGE_THRESHOLD);
            ce[3].y = f32((qe3.y - compute_local_contrast_h(1, 1, &neighbourhood)) > EDGE_THRESHOLD);

            // left
            ce[0].z = left_column.x;
            ce[1].z = ce[0].x;
            ce[2].z = left_column.y;
            ce[3].z = ce[2].x;

            // top
            ce[0].w = top_row.x;
            ce[1].w = top_row.y;
            ce[2].w = ce[0].y;
            ce[3].w = ce[1].y;

            for (var i: u32 = 0u; i < 4u; i++) {
                let local_pixel_pos = pixel_pos + qe_offsets(i);
                let edges = ce[i];

                // If there's at least one two-edge corner, this is a candidate
                // for simple or complex shape processing.
                let is_candidate = (edges.x * edges.y + edges.y * edges.z + edges.z * edges.w + edges.w * edges.x) != 0.0;
                if (is_candidate) {
                    let counter_index = atomicAdd(&control_buffer[1], 1u);
                    if counter_index < shape_candidates_length {
                        shape_candidates[counter_index] = (local_pixel_pos.x << 16u) | local_pixel_pos.y;
                    }
                }

                // Write out edges - we write out all, including empty pixels,
                // to make sure shape detection edge tracing doesn't continue
                // on previous frame's edges that no longer exist.
                out_edges[i] = pack_edges(edges);
            }
        }

        // Finally, write the edges!
        textureStore(edges,
            vec2<i32>(i32(pixel_pos.x/2u), i32(pixel_pos.y)),
            vec4<u32>((out_edges[1] << 4u) | out_edges[0], 0u, 0u, 0u)
        );
        textureStore(edges,
            vec2<i32>(i32(pixel_pos.x/2u), i32(pixel_pos.y + 1u)),
            vec4<u32>((out_edges[3] << 4u) | out_edges[2], 0u, 0u, 0u)
        );
    }
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

fn load_source_color(pixel_pos: vec2<u32>, offset: vec2<i32>) -> vec4<f32> {
    return textureLoad(input_texture, vec2<i32>(pixel_pos) + offset, 0);
}

/// Apply custom curve / processing to put input color (linear)
/// to convert it into the perceptional space / gamma.
fn process_color_for_edge_detect(color: vec3<f32>) -> vec3<f32> {
    // Just very roughly approximate RGB curve.
    return sqrt(color);
}

fn compute_edge(x: i32, y: i32, pixel_colors: ptr<private,array<vec3<f32>, 8>>) -> vec2<f32> {
    return vec2<f32>(
        edge_detect_color_calc_diff((*pixel_colors)[x + y * 3], (*pixel_colors)[x + 1 + y * 3]),
        edge_detect_color_calc_diff((*pixel_colors)[x + y * 3], (*pixel_colors)[x + (y + 1) * 3])
    );
}

fn edge_detect_color_calc_diff(color_a: vec3<f32>, color_b: vec3<f32>) -> f32 {
    let lum_weights = vec3<f32>(0.299, 0.587, 0.114);
    let diff = abs(color_a - color_b);
    return dot(diff, lum_weights);
}

fn load_quad_hv(addr: u32) -> mat4x2<f32> {
    let h = edges_h[addr];
    let v = edges_v[addr];
    let e00 = vec2<f32>(v.x, h.x);
    let e10 = vec2<f32>(v.y, h.y);
    let e01 = vec2<f32>(v.z, h.z);
    let e11 = vec2<f32>(v.w, h.w);
    return mat4x2<f32>(e00, e10, e01, e11);
}

fn compute_local_contrast_v(x: i32, y: i32, neighbourhood: ptr<private,array<mat4x2<f32>, 4>>) -> f32 {
    let max1 = max((*neighbourhood)[x + 1][y].y, (*neighbourhood)[x + 1][y + 1].y);
    let max2 = max((*neighbourhood)[x + 2][y].y, (*neighbourhood)[x + 2][y + 1].y);
    return max(max1, max2) * LOCAL_CONTRAST_ADAPTATION_AMOUNT;
}

fn compute_local_contrast_h(x: i32, y: i32, neighbourhood: ptr<private,array<mat4x2<f32>, 4>>) -> f32 {
    let max1 = max((*neighbourhood)[x][y + 1].x, (*neighbourhood)[x + 1][y + 1].x);
    let max2 = max((*neighbourhood)[x][y + 2].x, (*neighbourhood)[x + 1][y + 2].x);
    return max(max1, max2) * LOCAL_CONTRAST_ADAPTATION_AMOUNT;
}

/// How .rgba channels from the edge texture maps to pixel edges:
///
///                   A - 0x08               (A - there's an edge between us and a pixel above us)
///              |---------|                 (R - there's an edge between us and a pixel to the right)
///              |         |                 (G - there's an edge between us and a pixel at the bottom)
///     0x04 - B |  pixel  | R - 0x01        (B - there's an edge between us and a pixel to the left)
///              |         |
///              |_________|
///                   G - 0x02
fn pack_edges(edges: vec4<f32>) -> u32 {
    return u32(dot(edges, vec4<f32>(1.0, 2.0, 4.0, 8.0)));
}
