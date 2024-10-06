struct Constants {
    pointer_position: vec2<u32>,
}

@group(0) @binding(0) var texture: texture_2d<u32>;
@group(0) @binding(1) var<storage, read_write> buffer: array<u32>;
var<push_constant> constants: Constants;

@compute
@workgroup_size(1)
fn cs_main() {
    let texel_value = textureLoad(texture, constants.pointer_position, 0).rg;
    buffer[0] = texel_value.r;
    buffer[1] = texel_value.g;
}
