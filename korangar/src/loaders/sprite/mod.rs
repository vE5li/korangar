use std::collections::HashMap;
use std::sync::Arc;

use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_interface::elements::PrototypeElement;
use ragnarok_bytes::{ByteStream, FromBytes};
use ragnarok_formats::sprite::{PaletteColor, RgbaImageData, SpriteData};
use ragnarok_formats::version::InternalVersion;
use wgpu::{Device, Extent3d, Queue, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use super::FALLBACK_SPRITE_FILE;
use crate::graphics::Texture;
use crate::loaders::error::LoadError;
use crate::loaders::GameFileLoader;

#[derive(Clone, Debug, PrototypeElement)]
pub struct Sprite {
    #[hidden_element]
    pub textures: Vec<Arc<Texture>>,
    #[cfg(feature = "debug")]
    sprite_data: SpriteData,
}

#[derive(new)]
pub struct SpriteLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    #[new(default)]
    cache: HashMap<String, Arc<Sprite>>,
}

impl SpriteLoader {
    fn load(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Sprite>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load sprite from {}", path.magenta()));

        let bytes = game_file_loader.get(&format!("data\\sprite\\{path}")).map_err(LoadError::File)?;
        let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

        let sprite_data = match SpriteData::from_bytes(&mut byte_stream) {
            Ok(sprite_data) => sprite_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load sprite: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.get(FALLBACK_SPRITE_FILE, game_file_loader);
            }
        };

        #[cfg(feature = "debug")]
        let cloned_sprite_data = sprite_data.clone();

        let palette = sprite_data.palette.unwrap(); // unwrap_or_default() as soon as i know what
        // the default palette is

        let rgba_images/*: Vec<Arc<ImmutableImage>>*/ = sprite_data
            .rgba_image_data
            .into_iter();

        // TODO: Move this to an extension trait in `korangar_loaders`.
        pub fn color_bytes(palette: &PaletteColor, index: u8) -> [u8; 4] {
            let alpha = match index {
                0 => 0,
                _ => 255,
            };

            [palette.red, palette.green, palette.blue, alpha]
        }

        let palette_images = sprite_data.palette_image_data.into_iter().map(|image_data| {
            // decode palette image data if necessary
            let data: Vec<u8> = image_data
                .data
                .0
                .iter()
                .flat_map(|palette_index| color_bytes(&palette.colors[*palette_index as usize], *palette_index))
                .collect();

            RgbaImageData {
                width: image_data.width,
                height: image_data.height,
                data,
            }
        });

        let textures = rgba_images
            .chain(palette_images)
            .map(|image_data| {
                let texture = Texture::new_with_data(
                    &self.device,
                    &self.queue,
                    &TextureDescriptor {
                        label: Some(path),
                        size: Extent3d {
                            width: image_data.width as u32,
                            height: image_data.height as u32,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8UnormSrgb,
                        usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    &image_data.data,
                );
                Arc::new(texture)
            })
            .collect();

        let sprite = Arc::new(Sprite {
            textures,
            #[cfg(feature = "debug")]
            sprite_data: cloned_sprite_data,
        });

        self.cache.insert(path.to_string(), sprite.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Sprite>, LoadError> {
        match self.cache.get(path) {
            Some(sprite) => Ok(sprite.clone()),
            None => self.load(path, game_file_loader),
        }
    }
}
