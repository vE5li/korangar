struct Matrices {
    view_projection: mat4x4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) identifier: u32,
}

@group(0) @binding(0) var<uniform> matrices: Matrices;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) identifier: u32
) -> VertexOutput {
    var output: VertexOutput;
    output.position = matrices.view_projection * vec4<f32>(position, 1.0);
    output.identifier = identifier;
    return output;
}

@fragment
fn fs_main(@location(0) @interpolate(flat) identifier: u32) -> @location(0) u32 {
    return identifier;
}
