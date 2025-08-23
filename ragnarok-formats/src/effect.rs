use cgmath::Vector2;
use ragnarok_bytes::ByteConvertable;

use crate::signature::Signature;
use crate::version::{MajorFirst, Version};

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct TextureName {
    #[length(128)]
    pub name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Frame {
    pub frame_index: i32,
    pub frame_type: i32,
    pub offset: Vector2<f32>,
    pub uv: [f32; 8],
    pub xy: [f32; 8],
    pub texture_index: f32,
    pub animation_type: i32,
    pub delay: f32,
    pub angle: f32,
    pub color: [f32; 4],
    pub source_blend_factor: i32,
    pub destination_blend_factor: i32,
    pub mt_present: i32,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct LayerData {
    #[new_derive]
    pub texture_count: i32,
    #[repeating(texture_count)]
    pub texture_names: Vec<TextureName>,
    #[new_derive]
    pub frame_count: i32,
    #[repeating(frame_count)]
    pub frames: Vec<Frame>,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct EffectData {
    #[new_default]
    pub signature: Signature<b"STRM">,
    #[version]
    pub version: Version<MajorFirst>,
    #[new_default]
    pub _skip0: [u8; 2],
    pub frames_per_second: u32,
    pub max_key: u32,
    #[new_derive]
    pub layer_count: u32,
    #[new_default]
    pub _skip1: [u8; 16],
    #[repeating(layer_count)]
    pub layers: Vec<LayerData>,
}
