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

struct DispatchIndirectArgs {
    x: u32,
    y: u32,
    z: u32,
}

@group(0) @binding(1) var<storage, read_write> control_buffer: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> indirect_buffer: DispatchIndirectArgs;

const PROCESS_CANDIDATES_NUM_THREADS: u32 = 128;
const DEFERRED_APPLY_NUM_THREADS: u32 = 32;

@compute @workgroup_size(1, 1, 1)
fn cs_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    // Activated once on Dispatch(2, 1, 1)
    if (global_id.x == 1u) {
        // Get current count
        let shape_candidate_count = atomicLoad(&control_buffer[1]);

        // Write dispatch indirect arguments for process_candidates shader
        indirect_buffer.x = (shape_candidate_count + PROCESS_CANDIDATES_NUM_THREADS - 1u) / PROCESS_CANDIDATES_NUM_THREADS;
        indirect_buffer.y = 1u;
        indirect_buffer.z = 1u;

        // Write actual number of items to process in process_candidates shader
        atomicStore(&control_buffer[0], shape_candidate_count);
    }
    // Activated once on Dispatch(1, 2, 1)
    else if (global_id.y == 1u) {
        // Get current count
        let blend_location_count = atomicLoad(&control_buffer[2]);

        // Write dispatch indirect arguments for deferred_color_apply shader
        indirect_buffer.x = 1u;
        indirect_buffer.y = (blend_location_count + DEFERRED_APPLY_NUM_THREADS - 1u) / DEFERRED_APPLY_NUM_THREADS;
        indirect_buffer.z = 1u;

        // Write actual number of items to process in deferred_color_apply shader
        atomicStore(&control_buffer[0], blend_location_count);

        // Clear counters for next frame
        atomicStore(&control_buffer[1], 0u);  // Shape candidates counter
        atomicStore(&control_buffer[2], 0u);  // Blend locations counter
        atomicStore(&control_buffer[3], 0u);  // Deferred blend items counter
    }
}
