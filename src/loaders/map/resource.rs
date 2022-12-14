use cgmath::{Deg, Vector3};
use crate::graphics::{Color, Transform};
use crate::loaders::{ByteConvertable, ByteStream};
use crate::world::{EffectSource, LightSource, Object, SoundSource};

#[derive(Copy, Clone, Debug)]
pub enum ResourceType {
    Object,
    LightSource,
    SoundSource,
    EffectSource,
}

impl ResourceType {
    pub fn from(index: i32) -> Self {
        match index {
            1 => ResourceType::Object,
            2 => ResourceType::LightSource,
            3 => ResourceType::SoundSource,
            4 => ResourceType::EffectSource,
            _ => panic!("invalid object type {}", index),
        }
    }
}

pub struct MapResources {
    resources_amount: usize,
    pub objects: Vec<Object>,
    pub light_source: Vec<LightSource>,
    pub sound_source: Vec<SoundSource>,
    pub effect_source: Vec<EffectSource>,
}


impl ByteConvertable for MapResources {
    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        [1 as u8].to_vec()
    }

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        let resources_amount = byte_stream.integer32() as usize;

        let mut objects = Vec::new();
        let mut light_sources = Vec::new();
        let mut sound_sources = Vec::new();
        let mut effect_sources = Vec::new();

        for index in 0..resources_amount {
            let type_index = byte_stream.integer32();
            let resource_type = ResourceType::from(type_index);

            match resource_type {
                ResourceType::Object => {
                    if true {
                        //resource_version.equals_or_above(1, 6) {
                        let name = byte_stream.string(40);
                        let _animation_type = byte_stream.integer32();
                        let _animation_speed = byte_stream.float32();
                        let _block_type = byte_stream.integer32();
                        let model_name = byte_stream.string(80);
                        let _node_name = byte_stream.string(80);
                        let position = byte_stream.vector3_flipped();
                        let rotation = byte_stream.vector3();
                        let scale = byte_stream.vector3();

                        // offset the objects slightly to avoid depth buffer fighting
                        let position = position + Vector3::new(0.0, 0.0005, 0.0) * index as f32;

                        //let model = model_loader.get(game_file_loader, texture_loader, &model_name,
                        // reverse_order)?; // resolve with a map
                        let transform = Transform::from(position, rotation.map(Deg), scale);
                        let object = Object::new(Some(name), model_name, None, transform);

                        objects.push(object);
                    } else {
                        let model_name = byte_stream.string(80);
                        let _node_name = byte_stream.string(80);
                        let position = byte_stream.vector3_flipped();
                        let rotation = byte_stream.vector3();
                        let scale = byte_stream.vector3();

                        //let model = model_loader.get(game_file_loader, texture_loader, &model_name,
                        // reverse_order)?;
                        let transform = Transform::from(position, rotation.map(Deg), scale);
                        let object = Object::new(None, model_name, None, transform);
                        objects.push(object);
                    }
                }
                ResourceType::LightSource => {
                    let name = byte_stream.string(80);
                    let position = byte_stream.vector3_flipped();
                    let red = byte_stream.float32();
                    let green = byte_stream.float32();
                    let blue = byte_stream.float32();

                    let color = Color::rgb_f32(red, green, blue);
                    let range = byte_stream.float32();

                    light_sources.push(LightSource::new(name, position, color, range));
                }
                ResourceType::SoundSource => {
                    let name = byte_stream.string(80);
                    let sound_file = byte_stream.string(80);
                    let position = byte_stream.vector3_flipped();
                    let volume = byte_stream.float32();
                    let width = byte_stream.integer32();
                    let height = byte_stream.integer32();
                    let range = byte_stream.float32();

                    // let cycle = match resource_version.equals_or_above(2, 0) {
                    //     true => byte_stream.float32(),
                    //     false => 4.0,
                    // };

                    let cycle = byte_stream.float32();

                    sound_sources.push(SoundSource::new(
                        name,
                        sound_file,
                        position,
                        volume,
                        width as usize,
                        height as usize,
                        range,
                        cycle,
                    ));
                }
                ResourceType::EffectSource => {
                    let name = byte_stream.string(80);
                    let position = byte_stream.vector3_flipped();
                    let effect_type = byte_stream.integer32();
                    let emit_speed = byte_stream.float32();

                    let _param0 = byte_stream.float32();
                    let _param1 = byte_stream.float32();
                    let _param2 = byte_stream.float32();
                    let _param3 = byte_stream.float32();

                    effect_sources.push(EffectSource::new(name, position, effect_type as usize, emit_speed));
                }
            }
        }

        Self {
            resources_amount,
            objects,
            light_source: light_sources,
            sound_source: sound_sources,
            effect_source: effect_sources,
        }
    }
}
