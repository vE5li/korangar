use cgmath::{Matrix3, Point3, Quaternion, Vector2, Vector3};
use ragnarok_bytes::{
    ByteConvertable, ByteStream, ConversionError, ConversionResult, ConversionResultExt, FromBytes, FromBytesExt, ToBytes,
};

use crate::signature::Signature;
use crate::version::{InternalVersion, MajorFirst, Version};

/// A string that can either have a fixed lenght or be length prefixed, based on
/// the file format version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelString<const LENGTH: usize> {
    pub inner: String,
}

impl<const LENGTH: usize> FromBytes for ModelString<LENGTH> {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let inner = if byte_stream
            .get_metadata::<Self, Option<InternalVersion>>()?
            .ok_or(ConversionError::from_message("version not set"))?
            .equals_or_above(2, 2)
        {
            let length = u32::from_bytes(byte_stream).trace::<Self>()? as usize;
            let mut inner = String::from_n_bytes(byte_stream, length).trace::<Self>()?;
            // need to remove the last character for some reason
            inner.pop();
            inner
        } else {
            String::from_n_bytes(byte_stream, LENGTH).trace::<Self>()?
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
pub struct PositionKeyframeData {
    pub frame: u32,
    pub position: Point3<f32>,
}

#[derive(Clone, Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct RotationKeyframeData {
    pub frame: u32,
    pub quaternions: Quaternion<f32>,
}

#[derive(Debug, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct FaceData {
    pub vertex_position_indices: [u16; 3],
    pub texture_coordinate_indices: [u16; 3],
    pub texture_index: u16,
    pub padding: u16,
    pub two_sided: i32,
    pub smooth_group: i32,
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
    pub parent_node_name: ModelString<40>, // This is where 2.2 starts failing
    pub texture_count: u32,
    #[repeating(texture_count)]
    pub texture_indices: Vec<u32>,
    #[cfg_attr(feature = "interface", hidden_element)]
    pub offset_matrix: Matrix3<f32>,
    pub translation1: Vector3<f32>,
    pub translation2: Vector3<f32>,
    pub rotation_angle: f32,
    pub rotation_axis: Vector3<f32>,
    pub scale: Vector3<f32>,
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
    #[version_equals_or_above(2, 5)] // unsure what vesion this is supposed to be (must be > 1.5)
    #[new_derive]
    pub position_keyframe_count: Option<u32>,
    #[repeating_option(position_keyframe_count)]
    pub position_keyframes: Vec<PositionKeyframeData>,
    #[new_derive]
    pub rotation_keyframe_count: u32,
    #[repeating(rotation_keyframe_count)]
    pub rotation_keyframes: Vec<RotationKeyframeData>,
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
    pub reserved0: Option<[u8; 16]>,
    #[version_equals_or_above(2, 2)]
    #[new_default]
    pub reserved1: Option<[u8; 4]>,
    #[new_derive]
    pub texture_count: u32,
    #[repeating(texture_count)]
    pub texture_names: Vec<ModelString<40>>,
    #[version_equals_or_above(2, 2)]
    #[new_default]
    pub skip: Option<u32>,
    pub root_node_name: ModelString<40>,
    #[new_derive]
    pub node_count: u32,
    #[repeating(node_count)]
    pub nodes: Vec<NodeData>,
}
