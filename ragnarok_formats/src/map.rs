use cgmath::Vector3;
use ragnarok_bytes::{ByteConvertable, ByteStream, ConversionError, ConversionResult, ConversionResultExt, FromBytes};

use crate::color::{ColorBGRA, ColorRGB};
use crate::signature::Signature;
use crate::transform::Transform;
use crate::version::{InternalVersion, MajorFirst, Version};

#[derive(Clone, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[cfg_attr(feature = "interface", derive(korangar_interface::windows::PrototypeWindow))]
#[window_title("Map Viewer")]
#[window_class("map_viewer")]
pub struct MapData {
    pub signature: Signature<b"GRSW">,
    #[version]
    pub version: Version<MajorFirst>,
    #[version_equals_or_above(2, 5)]
    pub build_number: Option<i32>,
    #[version_equals_or_above(2, 2)]
    pub _unknown: Option<u8>,
    #[length_hint(40)]
    pub _ini_file: String,
    #[length_hint(40)]
    pub ground_file: String,
    #[length_hint(40)]
    pub gat_file: String,
    #[version_equals_or_above(1, 4)]
    #[length_hint(40)]
    pub _source_file: Option<String>,
    #[version_smaller(2, 6)]
    pub water_settings: Option<WaterSettings>,
    pub light_settings: LightSettings,
    #[version_equals_or_above(1, 6)]
    pub ground_top: Option<i32>,
    #[version_equals_or_above(1, 6)]
    pub ground_bottom: Option<i32>,
    #[version_equals_or_above(1, 6)]
    pub ground_left: Option<i32>,
    #[version_equals_or_above(1, 6)]
    pub ground_right: Option<i32>,
    // TODO: verify version
    //`#[version_equals_or_above(2, 6)]
    //pub quad_tree: QuadTree,
    pub resources: MapResources,
}

bitflags::bitflags! {
    #[derive(Debug)]
    pub struct TileFlags: u8 {
        const WALKABLE = 0b00000001;
        const WATER = 0b00000010;
        const SNIPABLE = 0b00000100;
        const CLIFF = 0b00001000;
    }
}

impl FromBytes for TileFlags {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        match byte_stream.byte::<Self>()? {
            0 => Ok(Self::WALKABLE),
            1 => Ok(Self::empty()),
            2 => Ok(Self::WATER),
            3 => Ok(Self::WATER | Self::WALKABLE),
            4 => Ok(Self::WATER | Self::SNIPABLE),
            5 => Ok(Self::CLIFF | Self::SNIPABLE),
            6 => Ok(Self::CLIFF),
            invalid => Err(ConversionError::from_message(format!("invalid tile type {invalid}"))),
        }
    }
}

#[derive(Debug, FromBytes)]
pub struct Tile {
    pub upper_left_height: f32,
    pub upper_right_height: f32,
    pub lower_left_height: f32,
    pub lower_right_height: f32,
    pub flags: TileFlags,
    pub unused: [u8; 3],
}

#[derive(FromBytes)]
pub struct GatData {
    pub signature: Signature<b"GRAT">,
    #[version]
    pub version: Version<MajorFirst>,
    pub map_width: i32,
    pub map_height: i32,
    #[repeating(self.map_width * self.map_height)]
    pub tiles: Vec<Tile>,
}

#[derive(FromBytes)]
pub struct GroundData {
    pub signature: Signature<b"GRGN">,
    #[version]
    pub version: Version<MajorFirst>,
    pub width: i32,
    pub height: i32,
    pub zoom: f32,
    pub texture_count: i32,
    pub texture_name_length: i32,
    #[repeating(self.texture_count)]
    #[length_hint(self.texture_name_length)]
    pub textures: Vec<String>,
    pub light_map_count: i32,
    pub light_map_width: i32,
    pub light_map_height: i32,
    pub light_map_cells_per_grid: i32,
    #[version_equals_or_above(1, 7)]
    #[length_hint(self.light_map_count * self.light_map_width * self.light_map_height * 4)]
    pub _skip: Option<Vec<u8>>,
    #[version_smaller(1, 7)]
    #[length_hint(self.light_map_count * 16)]
    pub _skip2: Option<Vec<u8>>,
    pub surface_count: i32,
    #[repeating(self.surface_count)]
    pub surfaces: Vec<Surface>,
    #[repeating(self.width as usize * self.height as usize)]
    pub ground_tiles: Vec<GroundTile>,
}

pub struct GroundTile {
    pub upper_left_height: f32,
    pub upper_right_height: f32,
    pub lower_left_height: f32,
    pub lower_right_height: f32,
    pub top_surface_index: i32,
    pub front_surface_index: i32,
    pub right_surface_index: i32,
}

impl FromBytes for GroundTile {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let upper_left_height = f32::from_bytes(byte_stream).trace::<Self>()?;
        let upper_right_height = f32::from_bytes(byte_stream).trace::<Self>()?;
        let lower_left_height = f32::from_bytes(byte_stream).trace::<Self>()?;
        let lower_right_height = f32::from_bytes(byte_stream).trace::<Self>()?;

        let version = byte_stream
            .get_metadata::<Self, Option<InternalVersion>>()?
            .ok_or(ConversionError::from_message("version not set"))?;

        let top_surface_index = match version.equals_or_above(1, 7) {
            true => i32::from_bytes(byte_stream).trace::<Self>()?,
            false => i16::from_bytes(byte_stream).trace::<Self>()? as i32,
        };

        let front_surface_index = match version.equals_or_above(1, 7) {
            true => i32::from_bytes(byte_stream).trace::<Self>()?,
            false => i16::from_bytes(byte_stream).trace::<Self>()? as i32,
        };

        let right_surface_index = match version.equals_or_above(1, 7) {
            true => i32::from_bytes(byte_stream).trace::<Self>()?,
            false => i16::from_bytes(byte_stream).trace::<Self>()? as i32,
        };

        Ok(Self {
            upper_left_height,
            upper_right_height,
            lower_left_height,
            lower_right_height,
            top_surface_index,
            front_surface_index,
            right_surface_index,
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SurfaceType {
    Front,
    Right,
    Top,
}

#[derive(FromBytes)]
pub struct Surface {
    pub u: [f32; 4],
    pub v: [f32; 4],
    pub texture_index: i16,
    pub light_map_index: i16,
    pub color: ColorBGRA,
}

#[derive(Copy, Clone, Debug)]
pub enum ResourceType {
    Object,
    LightSource,
    SoundSource,
    EffectSource,
}

impl FromBytes for ResourceType {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let index = i32::from_bytes(byte_stream).trace::<Self>()?;
        match index {
            1 => Ok(ResourceType::Object),
            2 => Ok(ResourceType::LightSource),
            3 => Ok(ResourceType::SoundSource),
            4 => Ok(ResourceType::EffectSource),
            _ => Err(ConversionError::from_message(format!("invalid object type {index}"))),
        }
    }
}

#[derive(Clone, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ObjectData {
    #[length_hint(40)]
    #[version_equals_or_above(1, 3)]
    pub name: Option<String>,
    #[version_equals_or_above(1, 3)]
    pub _animation_type: Option<i32>,
    #[version_equals_or_above(1, 3)]
    pub _animation_speed: Option<f32>,
    #[version_equals_or_above(1, 3)]
    pub _block_type: Option<i32>,
    // FIX: only if build_version >= 186
    #[version_equals_or_above(2, 6)]
    pub _unknown: Option<u8>,
    #[length_hint(80)]
    pub model_name: String,
    #[length_hint(80)]
    pub _node_name: String,
    pub transform: Transform,
}

#[derive(Clone)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct MapResources {
    resources_amount: usize,
    pub objects: Vec<ObjectData>,
    pub light_sources: Vec<LightSource>,
    pub sound_sources: Vec<SoundSource>,
    pub effect_sources: Vec<EffectSource>,
}

impl FromBytes for MapResources {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let resources_amount = i32::from_bytes(byte_stream).trace::<Self>()? as usize;

        let mut objects = Vec::new();
        let mut light_sources = Vec::new();
        let mut sound_sources = Vec::new();
        let mut effect_sources = Vec::new();

        for index in 0..resources_amount {
            let resource_type = ResourceType::from_bytes(byte_stream).trace::<Self>()?;

            match resource_type {
                ResourceType::Object => {
                    let mut object = ObjectData::from_bytes(byte_stream).trace::<Self>()?;
                    // offset the objects slightly to avoid depth buffer fighting
                    object.transform.position += Vector3::new(0.0, 0.0005, 0.0) * index as f32;
                    objects.push(object);
                }
                ResourceType::LightSource => {
                    let mut light_source = LightSource::from_bytes(byte_stream).trace::<Self>()?;
                    light_source.position.y = -light_source.position.y;
                    light_sources.push(light_source);
                }
                ResourceType::SoundSource => {
                    let mut sound_source = SoundSource::from_bytes(byte_stream).trace::<Self>()?;
                    sound_source.position.y = -sound_source.position.y;

                    if sound_source.cycle.is_none() {
                        sound_source.cycle = Some(4.0);
                    }

                    sound_sources.push(sound_source);
                }
                ResourceType::EffectSource => {
                    let mut effect_source = EffectSource::from_bytes(byte_stream).trace::<Self>()?;
                    effect_source.position.y = -effect_source.position.y;
                    effect_sources.push(effect_source);
                }
            }
        }

        Ok(Self {
            resources_amount,
            objects,
            light_sources,
            sound_sources,
            effect_sources,
        })
    }
}

#[derive(Clone, Debug, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct WaterSettings {
    #[version_equals_or_above(1, 3)]
    pub water_level: Option<f32>,
    #[version_equals_or_above(1, 8)]
    pub water_type: Option<i32>,
    #[version_equals_or_above(1, 8)]
    pub wave_height: Option<f32>,
    #[version_equals_or_above(1, 8)]
    pub wave_speed: Option<f32>,
    #[version_equals_or_above(1, 8)]
    pub wave_pitch: Option<f32>,
    #[version_equals_or_above(1, 9)]
    pub water_animation_speed: Option<u32>,
}

#[derive(Clone, Debug, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct LightSettings {
    #[version_equals_or_above(1, 5)]
    pub light_longitude: Option<i32>,
    #[version_equals_or_above(1, 5)]
    pub light_latitude: Option<i32>,
    #[version_equals_or_above(1, 5)]
    pub diffuse_color: Option<ColorRGB>,
    #[version_equals_or_above(1, 5)]
    pub ambient_color: Option<ColorRGB>,
    #[version_equals_or_above(1, 7)]
    pub light_intensity: Option<f32>,
}

#[derive(Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[cfg_attr(feature = "interface", derive(korangar_interface::windows::PrototypeWindow))]
#[window_title("Light Source")]
pub struct LightSource {
    #[length_hint(80)]
    pub name: String,
    pub position: Vector3<f32>,
    pub color: ColorRGB,
    pub range: f32,
}

#[derive(Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[cfg_attr(feature = "interface", derive(korangar_interface::windows::PrototypeWindow))]
#[window_title("Effect Source")]
pub struct EffectSource {
    #[length_hint(80)]
    pub name: String,
    pub position: Vector3<f32>,
    pub effect_type: u32, // TODO: fix this
    pub emit_speed: f32,
    pub _param0: f32,
    pub _param1: f32,
    pub _param2: f32,
    pub _param3: f32,
}

#[derive(Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[cfg_attr(feature = "interface", derive(korangar_interface::windows::PrototypeWindow))]
#[window_title("Sound Source")]
pub struct SoundSource {
    #[length_hint(80)]
    pub name: String,
    #[length_hint(80)]
    pub sound_file: String,
    pub position: Vector3<f32>,
    pub volume: f32,
    pub width: u32,
    pub height: u32,
    pub range: f32,
    #[version_equals_or_above(2, 0)]
    pub cycle: Option<f32>,
}
