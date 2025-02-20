use std::collections::VecDeque;

use cgmath::{Point3, Vector3};
use ragnarok_bytes::{ByteConvertable, ByteReader, ByteWriter, ConversionError, ConversionResult, ConversionResultExt, FromBytes, ToBytes};

use crate::color::{ColorBGRA, ColorRGB};
use crate::signature::Signature;
use crate::transform::Transform;
use crate::version::{InternalVersion, MajorFirst, Version};

#[derive(Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[cfg_attr(feature = "interface", derive(korangar_interface::windows::PrototypeWindow))]
#[cfg_attr(feature = "interface", window_title("Map Viewer"))]
#[cfg_attr(feature = "interface", window_class("map_viewer"))]
pub struct MapData {
    #[new_default]
    pub signature: Signature<b"GRSW">,
    #[version]
    pub version: Version<MajorFirst>,
    #[version_equals_or_above(2, 5)]
    pub build_number: Option<i32>,
    #[version_equals_or_above(2, 2)]
    pub _unknown: Option<u8>,
    #[length(40)]
    pub _ini_file: String,
    #[length(40)]
    pub ground_file: String,
    #[length(40)]
    pub gat_file: String,
    #[version_equals_or_above(1, 4)]
    #[length(40)]
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
    // TODO: Parse remaining fields
    pub resources: MapResources,
    #[version_equals_or_above(2, 1)]
    pub quadtree: Option<QuadTreeData>,
}

#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[derive(Clone)]
pub struct QuadTreeData {
    pub max: [f32; 3],
    pub min: [f32; 3],
    pub half_size: [f32; 3],
    pub center: [f32; 3],
    pub children: Vec<QuadTreeData>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TileFlags: u8 {
        const WALKABLE = 0b00000001;
        const WATER = 0b00000010;
        const SNIPABLE = 0b00000100;
        const CLIFF = 0b00001000;
    }
}

impl FromBytes for QuadTreeData {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        const MAX_DEPTH: usize = 5;
        const CHILD_COUNT: usize = 4;

        // Helper struct to keep node accesses readable.
        struct Node {
            data: QuadTreeData,
            parent_index: usize,
        }

        // Simulate a DFS using a stack.
        let mut stack = VecDeque::from([(0, 0)]);
        let mut nodes = Vec::new();

        while let Some((parent_index, depth)) = stack.pop_back() {
            let max = FromBytes::from_bytes(byte_reader).trace::<Self>()?;
            let min = FromBytes::from_bytes(byte_reader).trace::<Self>()?;
            let half_size = FromBytes::from_bytes(byte_reader).trace::<Self>()?;
            let center = FromBytes::from_bytes(byte_reader).trace::<Self>()?;

            let children = match depth < MAX_DEPTH {
                true => Vec::with_capacity(CHILD_COUNT),
                false => Vec::new(),
            };

            // Add next node lookups to the stack.
            if depth < MAX_DEPTH {
                let self_index = nodes.len();

                for _counter in 0..CHILD_COUNT {
                    stack.push_back((self_index, depth + 1));
                }
            }

            let data = QuadTreeData {
                max,
                min,
                half_size,
                center,
                children,
            };
            nodes.push(Node { data, parent_index });
        }

        // Go back to front and assign nodes to their parents. Node 0 is the root
        // node so we skip that.
        while nodes.len() > 1 {
            let node = nodes.pop().unwrap();

            let children = &mut nodes[node.parent_index].data.children;
            children.push(node.data);

            // We reverse the nodes once all of them are collected rather than inserting
            // them at the start of the vector to avoid unnecessary shifting of
            // other items. This is purely an optimization.
            if children.len() == 4 {
                children.reverse();
            }
        }

        let root_node = nodes.pop().unwrap().data;
        Ok(root_node)
    }
}

impl FromBytes for TileFlags {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        match <Self as bitflags::Flags>::Bits::from_bytes(byte_reader).trace::<Self>()? {
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

impl TryInto<<Self as bitflags::Flags>::Bits> for TileFlags {
    type Error = Box<ConversionError>;

    fn try_into(self) -> Result<<Self as bitflags::Flags>::Bits, Self::Error> {
        if self == Self::WALKABLE {
            Ok(0 as <Self as bitflags::Flags>::Bits)
        } else if self == Self::empty() {
            Ok(1 as <Self as bitflags::Flags>::Bits)
        } else if self == Self::WATER {
            Ok(2 as <Self as bitflags::Flags>::Bits)
        } else if self == Self::WATER | Self::WALKABLE {
            Ok(3 as <Self as bitflags::Flags>::Bits)
        } else if self == Self::WATER | Self::SNIPABLE {
            Ok(4 as <Self as bitflags::Flags>::Bits)
        } else if self == Self::CLIFF | Self::SNIPABLE {
            Ok(5 as <Self as bitflags::Flags>::Bits)
        } else if self == Self::CLIFF {
            Ok(6 as <Self as bitflags::Flags>::Bits)
        } else {
            Err(ConversionError::from_message(format!("invalid tile encoding {:?}", self)))
        }
    }
}

impl ToBytes for TileFlags {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        TryInto::<<Self as bitflags::Flags>::Bits>::try_into(*self)?
            .to_bytes(byte_writer)
            .trace::<Self>()
    }
}

#[derive(Debug, ByteConvertable)]
pub struct Tile {
    pub upper_left_height: f32,
    pub upper_right_height: f32,
    pub lower_left_height: f32,
    pub lower_right_height: f32,
    pub flags: TileFlags,
    #[new_default]
    pub unused: [u8; 3],
}

#[derive(ByteConvertable)]
pub struct GatData {
    #[new_default]
    pub signature: Signature<b"GRAT">,
    #[version]
    pub version: Version<MajorFirst>,
    pub map_width: i32,
    pub map_height: i32,
    #[repeating_expr(map_width as usize * map_height as usize)]
    pub tiles: Vec<Tile>,
}

#[derive(ByteConvertable)]
pub struct GroundData {
    #[new_default]
    pub signature: Signature<b"GRGN">,
    #[version]
    pub version: Version<MajorFirst>,
    pub width: i32,
    pub height: i32,
    pub zoom: f32,
    #[new_derive]
    pub texture_count: i32,
    pub texture_name_length: i32,
    #[repeating(texture_count)]
    #[length(texture_name_length)]
    pub textures: Vec<String>,
    pub light_map_count: i32,
    pub light_map_width: i32,
    pub light_map_height: i32,
    pub light_map_cells_per_grid: i32,
    #[version_equals_or_above(1, 7)]
    #[repeating_expr(light_map_count as usize * light_map_width as usize * light_map_height as usize * 4)]
    #[new_default]
    pub _skip: Option<Vec<u8>>,
    #[version_smaller(1, 7)]
    #[repeating_expr(light_map_count * 16)]
    #[new_default]
    pub _skip2: Option<Vec<u8>>,
    #[new_derive]
    pub surface_count: i32,
    #[repeating(surface_count)]
    pub surfaces: Vec<Surface>,
    #[repeating_expr(width as usize * height as usize)]
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
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let upper_left_height = f32::from_bytes(byte_reader).trace::<Self>()?;
        let upper_right_height = f32::from_bytes(byte_reader).trace::<Self>()?;
        let lower_left_height = f32::from_bytes(byte_reader).trace::<Self>()?;
        let lower_right_height = f32::from_bytes(byte_reader).trace::<Self>()?;

        let version = byte_reader
            .get_metadata::<Self, Option<InternalVersion>>()?
            .ok_or(ConversionError::from_message("version not set"))?;

        let top_surface_index = match version.equals_or_above(1, 7) {
            true => i32::from_bytes(byte_reader).trace::<Self>()?,
            false => i16::from_bytes(byte_reader).trace::<Self>()? as i32,
        };

        let front_surface_index = match version.equals_or_above(1, 7) {
            true => i32::from_bytes(byte_reader).trace::<Self>()?,
            false => i16::from_bytes(byte_reader).trace::<Self>()? as i32,
        };

        let right_surface_index = match version.equals_or_above(1, 7) {
            true => i32::from_bytes(byte_reader).trace::<Self>()?,
            false => i16::from_bytes(byte_reader).trace::<Self>()? as i32,
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

impl ToBytes for GroundTile {
    fn to_bytes(&self, _byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        panic!("GroundTile can not be serialized currently because it depends on a version requirement");
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
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let index = i32::from_bytes(byte_reader).trace::<Self>()?;
        match index {
            1 => Ok(ResourceType::Object),
            2 => Ok(ResourceType::LightSource),
            3 => Ok(ResourceType::SoundSource),
            4 => Ok(ResourceType::EffectSource),
            _ => Err(ConversionError::from_message(format!("invalid object type {index}"))),
        }
    }
}

#[derive(Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ObjectData {
    #[length(40)]
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
    #[new_default]
    pub _unknown: Option<u8>,
    #[length(80)]
    pub model_name: String,
    #[length(80)]
    pub _node_name: String,
    pub transform: Transform,
}

#[derive(Clone)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct MapResources {
    pub resources_amount: u32,
    pub objects: Vec<ObjectData>,
    pub light_sources: Vec<LightSource>,
    pub sound_sources: Vec<SoundSource>,
    pub effect_sources: Vec<EffectSource>,
}

impl MapResources {
    pub fn new(
        objects: Vec<ObjectData>,
        light_sources: Vec<LightSource>,
        sound_sources: Vec<SoundSource>,
        effect_sources: Vec<EffectSource>,
    ) -> Self {
        let resources_amount = (objects.len() + light_sources.len() + sound_sources.len() + effect_sources.len())
            .try_into()
            .expect("too many resources");

        Self {
            resources_amount,
            objects,
            light_sources,
            sound_sources,
            effect_sources,
        }
    }
}

impl FromBytes for MapResources {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let resources_amount = u32::from_bytes(byte_reader).trace::<Self>()?;

        let mut objects = Vec::new();
        let mut light_sources = Vec::new();
        let mut sound_sources = Vec::new();
        let mut effect_sources = Vec::new();

        for index in 0..resources_amount {
            let resource_type = ResourceType::from_bytes(byte_reader).trace::<Self>()?;

            match resource_type {
                ResourceType::Object => {
                    let mut object = ObjectData::from_bytes(byte_reader).trace::<Self>()?;
                    // offset the objects slightly to avoid depth buffer fighting
                    object.transform.position += Vector3::new(0.0, 0.0005, 0.0) * index as f32;
                    objects.push(object);
                }
                ResourceType::LightSource => {
                    let mut light_source = LightSource::from_bytes(byte_reader).trace::<Self>()?;
                    light_source.position.y = -light_source.position.y;

                    // Some light sources have color channels with values bigger than 1.0 (255), so
                    // we need to clamp them.
                    // TODO: Does this maybe have a special meaning?
                    light_source.color.clamp_color_channels();

                    light_sources.push(light_source);
                }
                ResourceType::SoundSource => {
                    let mut sound_source = SoundSource::from_bytes(byte_reader).trace::<Self>()?;
                    sound_source.position.y = -sound_source.position.y;

                    if sound_source.cycle.is_none() {
                        sound_source.cycle = Some(4.0);
                    }

                    sound_sources.push(sound_source);
                }
                ResourceType::EffectSource => {
                    let mut effect_source = EffectSource::from_bytes(byte_reader).trace::<Self>()?;
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

impl ToBytes for MapResources {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|write| {
            self.resources_amount.to_bytes(write)?;

            for object in &self.objects {
                object.to_bytes(write)?;
            }

            for light_source in &self.light_sources {
                light_source.to_bytes(write)?;
            }

            for sound_source in &self.sound_sources {
                sound_source.to_bytes(write)?;
            }

            for effect_source in &self.effect_sources {
                effect_source.to_bytes(write)?;
            }

            Ok(())
        })
    }
}

#[derive(Clone, Debug, ByteConvertable)]
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

#[derive(Clone, Debug, ByteConvertable)]
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
    pub shadow_map_alpha: Option<f32>,
}

#[derive(Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[cfg_attr(feature = "interface", derive(korangar_interface::windows::PrototypeWindow))]
#[cfg_attr(feature = "interface", window_title("Light Source"))]
pub struct LightSource {
    #[length(80)]
    pub name: String,
    pub position: Point3<f32>,
    pub color: ColorRGB,
    pub range: f32,
}

#[derive(Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[cfg_attr(feature = "interface", derive(korangar_interface::windows::PrototypeWindow))]
#[cfg_attr(feature = "interface", window_title("Effect Source"))]
pub struct EffectSource {
    #[length(80)]
    pub name: String,
    pub position: Point3<f32>,
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
#[cfg_attr(feature = "interface", window_title("Sound Source"))]
pub struct SoundSource {
    #[length(80)]
    pub name: String,
    #[length(80)]
    pub sound_file: String,
    pub position: Point3<f32>,
    pub volume: f32,
    pub width: u32,
    pub height: u32,
    pub range: f32,
    #[version_equals_or_above(2, 0)]
    pub cycle: Option<f32>,
}

#[cfg(test)]
mod conversion {
    // The way these tests are written might seem a bit strange, but it's to allow
    // changes to the `TileFlags` type without completely breaking the tests.
    //
    // The goal here is to verify the logic behind encoding and decoding, without
    // checking for specific values. This way, you can add more permutations to
    // `TileFlags` or even change the underlying data type without breaking the
    // tests.
    //
    // When adding new permutations `ENCODED_TILE_COUNT` needs to be adjusted.
    mod tile_flags {
        use bitflags::Flags;
        use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes, ToBytes};

        use crate::map::TileFlags;

        type EncodedType = <TileFlags as Flags>::Bits;

        const ENCODED_TILE_COUNT: usize = 7;

        #[derive(Default)]
        struct HitCounter {
            slots: [bool; ENCODED_TILE_COUNT],
        }

        impl HitCounter {
            fn register(&mut self, index: EncodedType) {
                assert!((index as usize) < ENCODED_TILE_COUNT, "index {index} is out of bounds");

                let slot = &mut self.slots[index as usize];

                assert!(!*slot, "index {index} was hit multiple times");

                *slot = true;
            }

            fn assert_all_slots_hit(self) {
                for (index, hit) in self.slots.into_iter().enumerate() {
                    if !hit {
                        panic!("index {index} was never hit");
                    }
                }
            }
        }

        // This test ensures that no more than one combination of flags is encoded
        // to a set of bytes. It also ensures that all possible bytes can be encoded.
        #[test]
        fn encode() {
            let mut hit_counter = HitCounter::default();

            let mut test = |flags: TileFlags| {
                let mut byte_writer = ByteWriter::new();

                if let Ok(_) = flags.to_bytes(&mut byte_writer) {
                    let bytes = byte_writer.into_inner();
                    let mut byte_reader = ByteReader::without_metadata(&bytes);
                    let index = EncodedType::from_bytes(&mut byte_reader).unwrap();
                    hit_counter.register(index);
                }
            };

            // Test with no bits set.
            test(TileFlags::empty());

            // All other possible premutations (but only once).
            for left in TileFlags::all().iter() {
                for right in TileFlags::all().iter() {
                    if left.bits() <= right.bits() {
                        test(left | right);
                    }
                }
            }

            hit_counter.assert_all_slots_hit();
        }

        // This test ensures that no more than one set of bytes is decoded
        // to the combination of flags. It also ensures that all possible flags can be
        // decoded.
        #[test]
        fn decode() {
            let mut hit_counter = HitCounter::default();

            for input in 0..EncodedType::MAX {
                let mut byte_writer = ByteWriter::new();

                input.to_bytes(&mut byte_writer).unwrap();
                let bytes = byte_writer.into_inner();

                let mut byte_reader = ByteReader::without_metadata(&bytes);

                if TileFlags::from_bytes(&mut byte_reader).is_ok() {
                    hit_counter.register(input)
                }
            }

            hit_counter.assert_all_slots_hit();
        }

        // Make sure that encoding and decoding agree.
        #[test]
        fn decode_encode() {
            for input in 0..EncodedType::MAX {
                let mut byte_writer = ByteWriter::new();

                input.to_bytes(&mut byte_writer).unwrap();
                let bytes = byte_writer.into_inner();

                let mut byte_reader = ByteReader::without_metadata(&bytes);

                if let Ok(decoded) = TileFlags::from_bytes(&mut byte_reader) {
                    let mut byte_writer = ByteWriter::new();

                    decoded.to_bytes(&mut byte_writer).unwrap();
                    let encoded = byte_writer.into_inner();

                    assert_eq!(encoded.as_slice(), bytes);
                }
            }
        }
    }
}
