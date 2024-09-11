struct Matrices {
    view_projection: mat4x4<f32>,
}

struct Constants {
    world: mat4x4<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> matrices: Matrices;

var<push_constant> constants: Constants;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return matrices.view_projection * constants.world * vec4<f32>(position, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return constants.color;
}
