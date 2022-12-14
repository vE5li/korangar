use procedural::*;

pub use super::resource::MapResources;
use crate::graphics::Color;
use crate::loaders::{ByteConvertable, ByteStream, Version};
use crate::world::{LightSettings, Tile, WaterSettings};

#[allow(dead_code)]
#[derive(ByteConvertable)]
pub struct MapData {
    #[version]
    pub version: Version,
    /// Ignored
    #[length_hint(40)]
    pub ini_file: String,
    #[length_hint(40)]
    pub ground_file: String,
    #[version_equals_or_above(1, 4)]
    #[length_hint(40)]
    pub gat_file: Option<String>,
    /// Ignored
    #[length_hint(40)]
    pub source_file: String,

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

#[allow(dead_code)]
#[derive(ByteConvertable)]
pub struct GatData {
    #[version]
    pub version: Version,
    pub map_width: i32,
    pub map_height: i32,
    #[repeating(self.map_width * self.map_height)]
    pub tiles: Vec<Tile>,
}

#[allow(dead_code)]
#[derive(ByteConvertable)]
pub struct GroundData {
    #[version]
    pub version: Version,
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
    #[length_hint(self.light_map_count * self.light_map_width * self.light_map_height * 4)]
    pub _skip: Vec<u8>,
    // // match ground_version.equals_or_above(1, 7) {
    // //     true => byte_stream.skip(light_map_count * light_map_dimensions * 4),
    // //     false => byte_stream.skip(light_map_count * 16),
    // // }
    pub surface_count: i32,
    #[repeating(self.surface_count)]
    pub surfaces: Vec<Surface>,
    #[repeating(self.width * self.height)]
    pub ground_tiles: Vec<GroundTile>,
}

#[derive(ByteConvertable)]
pub struct GroundTile {
    pub upper_left_height: f32,
    pub upper_right_height: f32,
    pub lower_left_height: f32,
    pub lower_right_height: f32,
    //handle i16 case
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

#[derive(Copy, Clone, Debug)]
pub enum SurfaceType {
    Front,
    Right,
    Top,
}

//#[derive(ByteConvertable)]
pub struct Surface {
    pub u: [f32; 4],
    pub v: [f32; 4],
    pub texture_index: i16,
    pub light_map_index: i16,
    pub color: Color,
}

impl ByteConvertable for Surface {
    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        [1 as u8].to_vec()
    }

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        let u = [
            byte_stream.float32(),
            byte_stream.float32(),
            byte_stream.float32(),
            byte_stream.float32(),
        ];
        let v = [
            byte_stream.float32(),
            byte_stream.float32(),
            byte_stream.float32(),
            byte_stream.float32(),
        ];

        let texture_index = byte_stream.integer16();
        let light_map_index = byte_stream.integer16();
        let color_bgra = byte_stream.slice(4);
        let color = Color::rgb(color_bgra[2], color_bgra[1], color_bgra[0]);

        Self {
            u,
            v,
            texture_index,
            light_map_index,
            color,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Heights {
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight,
}
