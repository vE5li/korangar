use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use cgmath::Deg;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_util::container::SimpleCache;
use korangar_util::FileLoader;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::effect::EffectData;
use ragnarok_formats::version::InternalVersion;
use wgpu::BlendFactor;

use super::error::LoadError;
use super::{ImageType, TextureLoader};
use crate::graphics::Color;
use crate::loaders::GameFileLoader;
use crate::world::{AnimationType, Effect, Frame, FrameType, Layer, MultiTexturePresent};

const MAX_CACHE_COUNT: u32 = 512;
const MAX_CACHE_SIZE: usize = 64 * 1024 * 1024;

pub struct EffectLoader {
    game_file_loader: Arc<GameFileLoader>,
    cache: Mutex<SimpleCache<String, Arc<Effect>>>,
}

impl EffectLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>) -> Self {
        Self {
            game_file_loader,
            cache: Mutex::new(SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            )),
        }
    }

    fn load(&self, path: &str, texture_loader: &TextureLoader) -> Result<Arc<Effect>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load effect from {}", path.magenta()));

        let bytes = self
            .game_file_loader
            .get(&format!("data\\texture\\effect\\{path}"))
            .map_err(LoadError::File)?;
        let mut byte_reader: ByteReader<Option<InternalVersion>> = ByteReader::with_default_metadata(&bytes);

        // TODO: Add fallback
        let effect_data = EffectData::from_bytes(&mut byte_reader).map_err(LoadError::Conversion)?;

        let prefix = match path.chars().rev().position(|character| character == '\\') {
            Some(offset) => path.split_at(path.len() - offset).0,
            None => "",
        };

        let effect = Arc::new(Effect::new(
            effect_data.frames_per_second as usize,
            effect_data.max_key as usize,
            effect_data
                .layers
                .into_iter()
                .map(|layer_data| {
                    let mut previous_source_blend_factor = None;
                    let mut previous_destination_blend_factor = None;

                    Layer::new(
                        layer_data
                            .texture_names
                            .into_iter()
                            .map(|name| {
                                let path = format!("effect\\{}{}", prefix, name.name);
                                texture_loader.get_or_load(&path, ImageType::Color).unwrap()
                            })
                            .collect(),
                        {
                            let frame_count = layer_data.frames.len();
                            let mut map = Vec::with_capacity(frame_count);
                            let mut list_index = 0;

                            if frame_count > 0 {
                                let mut previous = None;

                                for _ in 0..layer_data.frames[0].frame_index {
                                    map.push(None);
                                    list_index += 1;
                                }

                                for (index, frame) in layer_data.frames.iter().skip(1).enumerate() {
                                    for _ in list_index..frame.frame_index as usize {
                                        map.push(previous);
                                        list_index += 1;
                                    }

                                    previous = Some(index);
                                }

                                // TODO: conditional
                                map.push(previous);
                                list_index += 1;
                            }

                            for _ in list_index..effect_data.max_key as usize {
                                map.push(None)
                            }

                            map
                        },
                        layer_data
                            .frames
                            .into_iter()
                            .map(|frame| {
                                let source_blend_factor = parse_blend_factor(frame.source_blend_factor, previous_source_blend_factor, true);
                                previous_source_blend_factor = Some(source_blend_factor);

                                let destination_blend_factor =
                                    parse_blend_factor(frame.destination_blend_factor, previous_destination_blend_factor, false);
                                previous_destination_blend_factor = Some(destination_blend_factor);

                                let animation_type = parse_animation_type(frame.animation_type);
                                let frame_type = parse_frame_type(frame.frame_type);
                                let mt_present = parse_mt_present(frame.mt_present);

                                Frame::new(
                                    frame.frame_index as usize,
                                    frame_type,
                                    frame.offset,
                                    frame.uv,
                                    frame.xy,
                                    frame.texture_index as usize,
                                    animation_type,
                                    frame.delay,
                                    Deg(frame.angle / (1024.0 / 360.0)).into(),
                                    Color::rgba(
                                        frame.color[0] / 255.0,
                                        frame.color[1] / 255.0,
                                        frame.color[2] / 255.0,
                                        frame.color[3] / 255.0,
                                    ),
                                    source_blend_factor,
                                    destination_blend_factor,
                                    mt_present,
                                )
                            })
                            .collect(),
                    )
                })
                .collect(),
        ));

        self.cache.lock().unwrap().insert(path.to_string(), effect.clone()).unwrap();

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(effect)
    }

    pub fn get_or_load(&self, path: &str, texture_loader: &TextureLoader) -> Result<Arc<Effect>, LoadError> {
        let mut lock = self.cache.lock().unwrap();
        match lock.get(path) {
            Some(effect) => Ok(effect.clone()),
            None => {
                // We need to drop to avoid a deadlock here.
                drop(lock);
                self.load(path, texture_loader)
            }
        }
    }
}

fn parse_blend_factor(value: i32, previous: Option<BlendFactor>, is_source: bool) -> BlendFactor {
    match value {
        0 => previous.unwrap(),
        1 => BlendFactor::Zero,
        2 => BlendFactor::One,
        3 => BlendFactor::Src,
        4 => BlendFactor::OneMinusSrc,
        5 => BlendFactor::SrcAlpha,
        6 => BlendFactor::OneMinusSrcAlpha,
        7 => BlendFactor::DstAlpha,
        8 => BlendFactor::OneMinusDstAlpha,
        9 => BlendFactor::Dst,
        10 => BlendFactor::OneMinusDst,
        11 => BlendFactor::SrcAlphaSaturated,
        // D3DBLEND_BOTHSRCALPHA
        //
        // Obsolete. Starting with DirectX 6, you can achieve the same effect
        // by setting the source and destination blend factors to D3DBLEND_SRCALPHA
        // and D3DBLEND_INVSRCALPHA in separate calls.
        12 if is_source => BlendFactor::SrcAlpha,
        12 if !is_source => BlendFactor::OneMinusSrcAlpha,
        _ => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] unknown blend factor found in frame data: {value}", "error".red());
            BlendFactor::Zero
        }
    }
}

fn parse_animation_type(value: i32) -> AnimationType {
    match value {
        0 => AnimationType::Type0,
        1 => AnimationType::Type1,
        2 => AnimationType::Type2,
        3 => AnimationType::Type3,
        _ => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] unknown animation type found in frame data: {value}", "error".red());
            AnimationType::Type1
        }
    }
}

fn parse_frame_type(value: i32) -> FrameType {
    match value {
        0 => FrameType::Basic,
        1 => FrameType::Morphing,
        _ => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] unknown frame type found in frame data: {value}", "error".red());
            FrameType::Basic
        }
    }
}

fn parse_mt_present(value: i32) -> MultiTexturePresent {
    match value {
        0 => MultiTexturePresent::None,
        _ => {
            #[cfg(feature = "debug")]
            print_debug!("[{}] unknown multi texture present found in frame data: {value}", "error".red());
            MultiTexturePresent::None
        }
    }
}
