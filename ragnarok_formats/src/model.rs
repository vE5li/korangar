use cgmath::{Matrix3, Point3, Quaternion, Vector2, Vector3};
use ragnarok_bytes::{
    ByteConvertable, ByteReader, ConversionError, ConversionResult, ConversionResultExt, FromBytes, FromBytesExt, ToBytes,
};

use crate::signature::Signature;
use crate::version::{InternalVersion, MajorFirst, Version};

/// A string that can either have a fixed length or be length prefixed, based on
/// the file format version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelString<const LENGTH: usize> {
    pub inner: String,
}

impl<const LENGTH: usize> FromBytes for ModelString<LENGTH> {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let inner = if byte_reader
            .get_metadata::<Self, Option<InternalVersion>>()?
            .ok_or(ConversionError::from_message("version not set"))?
            .equals_or_above(2, 2)
        {
            let length = u32::from_bytes(byte_reader).trace::<Self>()? as usize;
            let mut inner = String::from_n_bytes(byte_reader, length).trace::<Self>()?;
            // need to remove the last character for some reason
            inner.pop();
            inner
        } else {
            String::from_n_bytes(byte_reader, LENGTH).trace::<Self>()?
        };

        Ok(Self { inner })
    }
}

impl<const LENGTH: usize> ToBytes for ModelString<LENGTH> {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        panic!("ModelString can not be serialized currently because it depends on a version requirement");
    }
}

impl<const LENGTH: usize> AsRef<str> for ModelString<LENGTH> {
    fn as_ref(&self) -> &str {
        self.inner.as_str()
    }
}

#[cfg(feature = "interface")]
impl<App, const LENGTH: usize> korangar_interface::elements::PrototypeElement<App> for ModelString<LENGTH>
where
    App: korangar_interface::application::Application,
{
    fn to_element(&self, display: String) -> korangar_interface::elements::ElementCell<App> {
        self.inner.to_element(display)
    }
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ScaleKeyframeData {
    pub frame: u32,
    pub scale: Vector3<f32>,
    reserved: f32,
}
#[derive(Clone, Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct RotationKeyframeData {
    pub frame: u32,
    pub quaternions: Quaternion<f32>,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct TranslationKeyframeData {
    pub frame: u32,
    pub translation: Vector3<f32>,
    reserved: f32,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct TexturesKeyframeData {
    pub texture_index: u32,
    #[new_derive]
    pub texture_keyframe_count: u32,
    #[repeating(texture_keyframe_count)]
    pub texture_keyframes: Vec<TextureKeyframeData>,
}

/// List of texture operation types.
/// See: https://rathena.org/board/topic/127587-rsm2-file-format/
#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum TextureOperation {
    /// Texture translation on the X axis. The texture is tiled.
    TranslationX,
    /// Texture translation on the Y axis. The texture is tiled.
    TranslationY,
    /// Texture multiplication on the X axis. The texture is tiled.
    ScaleX,
    /// Texture multiplication on the Y axis. The texture is tiled
    ScaleY,
    /// Texture rotation around (0, 0). The texture is not tiled.
    Rotation,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct TextureKeyframeData {
    pub operation_type: TextureOperation,
    #[new_derive]
    pub frame_count: u32,
    #[repeating(frame_count)]
    pub texture_frames: Vec<TextureFrameData>,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct TextureFrameData {
    pub frame: u32,
    pub operation_value: f32,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct FaceData {
    #[version_equals_or_above(2, 2)]
    pub length: Option<u32>,
    pub vertex_position_indices: [u16; 3],
    pub texture_coordinate_indices: [u16; 3],
    pub texture_index: u16,
    pub padding: u16,
    pub two_sided: i32,
    pub smooth_group: i32,
    #[version_equals_or_above(2, 2)]
    #[repeating_expr((length.unwrap() as usize).saturating_sub(24) / 4)]
    pub smooth_group_extra: Option<Vec<i32>>,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct TextureCoordinateData {
    #[version_equals_or_above(1, 2)]
    pub color: Option<u32>,
    pub coordinates: Vector2<f32>, // possibly wrong if version < 1.2
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct NodeData {
    pub node_name: ModelString<40>,
    pub parent_node_name: ModelString<40>,
    #[version_smaller(2, 3)]
    #[new_derive]
    pub texture_count: Option<u32>,
    #[repeating_option(texture_count)]
    pub texture_indices: Vec<u32>,
    #[version_equals_or_above(2, 3)]
    #[new_derive]
    pub texture_name_count: Option<u32>,
    #[repeating_option(texture_name_count)]
    pub texture_names: Vec<ModelString<40>>,
    #[cfg_attr(feature = "interface", hidden_element)]
    pub offset_matrix: Matrix3<f32>,
    #[version_smaller(2, 2)]
    pub translation1: Option<Vector3<f32>>,
    pub translation2: Vector3<f32>,
    #[version_smaller(2, 2)]
    pub rotation_angle: Option<f32>,
    #[version_smaller(2, 2)]
    pub rotation_axis: Option<Vector3<f32>>,
    #[version_smaller(2, 2)]
    pub scale: Option<Vector3<f32>>,
    #[new_derive]
    pub vertex_position_count: u32,
    #[repeating(vertex_position_count)]
    pub vertex_positions: Vec<Point3<f32>>,
    #[new_derive]
    pub texture_coordinate_count: u32,
    #[repeating(texture_coordinate_count)]
    pub texture_coordinates: Vec<TextureCoordinateData>,
    #[new_derive]
    pub face_count: u32,
    #[repeating(face_count)]
    pub faces: Vec<FaceData>,
    #[version_equals_or_above(1, 6)]
    #[new_derive]
    pub scale_keyframe_count: Option<u32>,
    #[repeating_option(scale_keyframe_count)]
    pub scale_keyframes: Vec<ScaleKeyframeData>,
    #[new_derive]
    pub rotation_keyframe_count: u32,
    #[repeating(rotation_keyframe_count)]
    pub rotation_keyframes: Vec<RotationKeyframeData>,
    #[version_equals_or_above(2, 2)]
    #[new_derive]
    pub translation_keyframe_count: Option<u32>,
    #[repeating_option(translation_keyframe_count)]
    pub translation_keyframes: Vec<TranslationKeyframeData>,
    #[version_equals_or_above(2, 3)]
    #[new_derive]
    pub textures_keyframe_count: Option<u32>,
    #[repeating_option(textures_keyframe_count)]
    pub textures_keyframes: Vec<TexturesKeyframeData>,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ModelData {
    #[new_default]
    pub signature: Signature<b"GRSM">,
    #[version]
    pub version: Version<MajorFirst>,
    pub animation_length: u32,
    pub shade_type: u32,
    #[version_equals_or_above(1, 4)]
    pub alpha: Option<u8>,
    #[version_smaller(2, 2)]
    #[new_default]
    pub reserved: Option<[u8; 16]>,
    #[version_equals_or_above(2, 2)]
    pub frames_per_second: Option<f32>,
    #[version_smaller(2, 3)]
    #[new_derive]
    pub texture_count: Option<u32>,
    #[repeating_option(texture_count)]
    pub texture_names: Vec<ModelString<40>>,
    #[version_smaller(2, 2)]
    pub root_node_name: Option<ModelString<40>>,
    #[version_equals_or_above(2, 2)]
    #[new_derive]
    pub root_node_count: Option<u32>,
    #[repeating_option(root_node_count)]
    pub root_node_names: Vec<ModelString<40>>,
    #[new_derive]
    pub node_count: u32,
    #[repeating(node_count)]
    pub nodes: Vec<NodeData>,
}
