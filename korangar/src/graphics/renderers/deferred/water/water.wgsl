struct Matrices {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
}

struct Constants {
    wave_offset: f32,
}

@group(0) @binding(0) var<uniform> matrices: Matrices;

var<push_constant> constants: Constants;

@vertex
fn vs_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    let adjusted_position = vec3<f32>(
        position.x,
        position.y + sin(constants.wave_offset + position.x + position.z),
        position.z
    );
    return matrices.projection * matrices.view * vec4<f32>(adjusted_position, 1.0);
}

@fragment
fn fs_main() -> @location(2) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 0.0);
}
