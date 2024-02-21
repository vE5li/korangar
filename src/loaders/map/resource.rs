use cgmath::Vector3;
use procedural::{FromBytes, Named, PrototypeElement};

use crate::graphics::{ColorRGB, Transform};
use crate::loaders::{conversion_result, ByteStream, ConversionError, FromBytes};
use crate::world::{EffectSource, LightSource, SoundSource};

#[derive(Copy, Clone, Debug, Named)]
pub enum ResourceType {
    Object,
    LightSource,
    SoundSource,
    EffectSource,
}

impl FromBytes for ResourceType {
    fn from_bytes(byte_stream: &mut ByteStream) -> Result<Self, Box<ConversionError>> {
        let index = conversion_result::<Self, _>(i32::from_bytes(byte_stream))?;
        match index {
            1 => Ok(ResourceType::Object),
            2 => Ok(ResourceType::LightSource),
            3 => Ok(ResourceType::SoundSource),
            4 => Ok(ResourceType::EffectSource),
            _ => Err(ConversionError::from_message(format!("invalid object type {index}"))),
        }
    }
}

#[derive(Clone, Named, FromBytes, PrototypeElement)]
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

impl ObjectData {
    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.transform.position += offset;
    }
}

#[allow(dead_code)]
#[derive(Clone, Named, PrototypeElement)]
pub struct MapResources {
    resources_amount: usize,
    pub objects: Vec<ObjectData>,
    pub light_sources: Vec<LightSource>,
    pub sound_sources: Vec<SoundSource>,
    pub effect_sources: Vec<EffectSource>,
}

impl FromBytes for MapResources {
    fn from_bytes(byte_stream: &mut ByteStream) -> Result<Self, Box<ConversionError>> {
        let resources_amount = conversion_result::<Self, _>(i32::from_bytes(byte_stream))? as usize;

        let mut objects = Vec::new();
        let mut light_sources = Vec::new();
        let mut sound_sources = Vec::new();
        let mut effect_sources = Vec::new();

        for index in 0..resources_amount {
            let resource_type = conversion_result::<Self, _>(ResourceType::from_bytes(byte_stream))?;

            match resource_type {
                ResourceType::Object => {
                    let mut object = conversion_result::<Self, _>(ObjectData::from_bytes(byte_stream))?;
                    // offset the objects slightly to avoid depth buffer fighting
                    object.transform.position += Vector3::new(0.0, 0.0005, 0.0) * index as f32;
                    objects.push(object);
                }
                ResourceType::LightSource => {
                    let mut light_source = conversion_result::<Self, _>(LightSource::from_bytes(byte_stream))?;
                    light_source.position.y = -light_source.position.y;
                    light_sources.push(light_source);
                }
                ResourceType::SoundSource => {
                    let mut sound_source = conversion_result::<Self, _>(SoundSource::from_bytes(byte_stream))?;
                    sound_source.position.y = -sound_source.position.y;

                    if sound_source.cycle.is_none() {
                        sound_source.cycle = Some(4.0);
                    }

                    sound_sources.push(sound_source);
                }
                ResourceType::EffectSource => {
                    let mut effect_source = conversion_result::<Self, _>(EffectSource::from_bytes(byte_stream))?;
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

#[derive(Clone, Debug, Named, FromBytes, PrototypeElement)]
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

#[derive(Clone, Debug, Named, FromBytes, PrototypeElement)]
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
