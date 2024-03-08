use procedural::*;
use ragnarok_bytes::{ByteStream, ConversionError, ConversionResult, ConversionResultExt, FromBytes};

pub use super::resource::MapResources;
use crate::graphics::ColorBGRA;
use crate::loaders::map::resource::{LightSettings, WaterSettings};
use crate::loaders::version::InternalVersion;
use crate::loaders::{MajorFirst, Version};
use crate::world::Tile;

#[derive(Clone, FromBytes, PrototypeElement, PrototypeWindow)]
#[window_title("Map Viewer")]
#[window_class("map_viewer")]
pub struct MapData {
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

#[derive(FromBytes)]
pub struct GatData {
    #[version]
    pub version: Version<MajorFirst>,
    pub map_width: i32,
    pub map_height: i32,
    #[repeating(self.map_width * self.map_height)]
    pub tiles: Vec<Tile>,
}

#[derive(FromBytes)]
pub struct GroundData {
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

impl GroundTile {
    pub fn get_lowest_point(&self) -> f32 {
        [
            self.lower_right_height,
            self.lower_left_height,
            self.upper_left_height,
            self.lower_right_height,
        ]
        .into_iter()
        .reduce(f32::max)
        .unwrap()
    }
}

impl FromBytes for GroundTile {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
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
