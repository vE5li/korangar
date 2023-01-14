use procedural::*;

pub use super::resource::MapResources;
use crate::graphics::ColorBGR;
use crate::loaders::map::resource::{LightSettings, WaterSettings};
use crate::loaders::{ByteConvertable, ByteStream, MajorFirst, Version};
use crate::world::Tile;

#[derive(ByteConvertable)]
pub struct MapData {
    #[version]
    pub version: Version<MajorFirst>,
    #[length_hint(40)]
    pub _ini_file: String,
    #[length_hint(40)]
    pub ground_file: String,
    #[version_equals_or_above(1, 4)]
    #[length_hint(40)]
    pub gat_file: Option<String>,
    #[length_hint(40)]
    pub _source_file: String,
    pub water_settings: WaterSettings,
    pub light_settings: LightSettings,
    #[version_equals_or_above(1, 6)]
    pub ground_top: Option<i32>,
    #[version_equals_or_above(1, 6)]
    pub ground_bottom: Option<i32>,
    #[version_equals_or_above(1, 6)]
    pub ground_left: Option<i32>,
    #[version_equals_or_above(1, 6)]
    pub ground_right: Option<i32>,
    pub resources: MapResources,
}

#[derive(ByteConvertable)]
pub struct GatData {
    #[version]
    pub version: Version<MajorFirst>,
    pub map_width: i32,
    pub map_height: i32,
    #[repeating(self.map_width * self.map_height)]
    pub tiles: Vec<Tile>,
}

#[derive(ByteConvertable)]
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
    #[repeating(self.width * self.height)]
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
        f32::max(
            self.lower_right_height,
            f32::max(
                self.lower_left_height,
                f32::max(self.upper_left_height, self.upper_right_height),
            ),
        )
    }
}

impl ByteConvertable for GroundTile {
    fn from_bytes(byte_stream: &mut ByteStream, _: Option<usize>) -> Self {
        let upper_left_height = byte_stream.float32();
        let upper_right_height = byte_stream.float32();
        let lower_left_height = byte_stream.float32();
        let lower_right_height = byte_stream.float32();

        let top_surface_index = match byte_stream.get_version().equals_or_above(1, 7) {
            true => byte_stream.integer32(),
            false => byte_stream.integer16() as i32,
        };

        let front_surface_index = match byte_stream.get_version().equals_or_above(1, 7) {
            true => byte_stream.integer32(),
            false => byte_stream.integer16() as i32,
        };

        let right_surface_index = match byte_stream.get_version().equals_or_above(1, 7) {
            true => byte_stream.integer32(),
            false => byte_stream.integer16() as i32,
        };

        Self {
            upper_left_height,
            upper_right_height,
            lower_left_height,
            lower_right_height,
            top_surface_index,
            front_surface_index,
            right_surface_index,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SurfaceType {
    Front,
    Right,
    Top,
}

#[derive(ByteConvertable)]
pub struct Surface {
    pub u: [f32; 4],
    pub v: [f32; 4],
    pub texture_index: i16,
    pub light_map_index: i16,
    pub color: ColorBGR,
}
