use cgmath::Vector2;
use ragnarok_bytes::FromBytes;

use crate::signature::Signature;
use crate::version::{MajorFirst, Version};

#[derive(Debug, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct TextureName {
    #[length_hint(128)]
    pub name: String,
}

#[derive(Debug, Clone, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
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
    // Needs to actually set the attachment blend mode of the source alpha
    pub source_alpha: i32,
    // Needs to actually set the attachment blend mode of the destination alpha
    pub destination_alpha: i32,
    pub mt_present: i32,
}

#[derive(Debug, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct LayerData {
    pub texture_count: i32,
    #[repeating(self.texture_count)]
    pub texture_names: Vec<TextureName>,
    pub frame_count: i32,
    #[repeating(self.frame_count)]
    pub frames: Vec<Frame>,
}

#[derive(Debug, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct EffectData {
    pub signature: Signature<b"STRM">,
    #[version]
    pub version: Version<MajorFirst>,
    pub _skip0: [u8; 2],
    pub frames_per_second: u32,
    pub max_key: u32,
    pub layer_count: u32,
    pub _skip1: [u8; 16],
    #[repeating(self.layer_count)]
    pub layers: Vec<LayerData>,
}
