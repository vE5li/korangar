use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;

#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_interface::elements::PrototypeElement;
use korangar_util::container::{Cacheable, SimpleCache};
use korangar_util::FileLoader;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::sprite::{PaletteColor, RgbaImageData, SpriteData};
use ragnarok_formats::version::InternalVersion;
use wgpu::{Device, Extent3d, Queue, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use super::FALLBACK_SPRITE_FILE;
use crate::graphics::Texture;
use crate::loaders::error::LoadError;
use crate::loaders::GameFileLoader;

const MAX_CACHE_COUNT: u32 = 512;
const MAX_CACHE_SIZE: usize = 512 * 1024 * 1024;

#[derive(Clone, Debug, PrototypeElement)]
pub struct Sprite {
    pub palette_size: usize,
    #[hidden_element]
    pub textures: Vec<Arc<Texture>>,
    #[cfg(feature = "debug")]
    sprite_data: SpriteData,
}

impl Cacheable for Sprite {
    fn size(&self) -> usize {
        self.textures.iter().map(|t| t.get_byte_size()).sum()
    }
}

pub struct SpriteLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    game_file_loader: Arc<GameFileLoader>,
    cache: SimpleCache<String, Arc<Sprite>>,
}

impl SpriteLoader {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, game_file_loader: Arc<GameFileLoader>) -> Self {
        Self {
            device,
            queue,
            game_file_loader,
            cache: SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            ),
        }
    }

    fn load(&mut self, path: &str) -> Result<Arc<Sprite>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load sprite from {}", path.magenta()));

        let bytes = match self.game_file_loader.get(&format!("data\\sprite\\{path}")) {
            Ok(bytes) => bytes,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load sprite: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.get(FALLBACK_SPRITE_FILE);
            }
        };
        let mut byte_reader: ByteReader<Option<InternalVersion>> = ByteReader::without_metadata(&bytes);

        let sprite_data = match SpriteData::from_bytes(&mut byte_reader) {
            Ok(sprite_data) => sprite_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load sprite: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.get(FALLBACK_SPRITE_FILE);
            }
        };

        #[cfg(feature = "debug")]
        let cloned_sprite_data = sprite_data.clone();

        let palette = sprite_data.palette.unwrap(); // unwrap_or_default() as soon as i know what

        let rgba_images: Vec<RgbaImageData> = sprite_data
            .rgba_image_data
            .iter()
            .map(|image_data| {
                // Revert the rows, the image is flipped upside down
                // Convert the pixel from ABGR format to RGBA format
                let width = image_data.width;
                let data = image_data
                    .data
                    .chunks_exact(4 * width as usize)
                    .rev()
                    .flat_map(|pixels| {
                        pixels
                            .chunks_exact(4)
                            .flat_map(|pixel| [pixel[3], pixel[2], pixel[1], pixel[0]])
                            .collect::<Vec<u8>>()
                    })
                    .collect();

                RgbaImageData {
                    width: image_data.width,
                    height: image_data.height,
                    data,
                }
            })
            .collect();

        // TODO: Move this to an extension trait in `korangar_loaders`.
        pub fn color_bytes(palette: &PaletteColor, index: u8) -> [u8; 4] {
            let alpha = match index {
                0 => 0,
                _ => 255,
            };

            [palette.red, palette.green, palette.blue, alpha]
        }

        let palette_images = sprite_data.palette_image_data.iter().map(|image_data| {
            // Decode palette image data if necessary
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
        let palette_size = palette_images.len();

        let textures = palette_images
            .chain(rgba_images)
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
            palette_size,
            textures,
            #[cfg(feature = "debug")]
            sprite_data: cloned_sprite_data,
        });
        let _ = self.cache.insert(path.to_string(), sprite.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get(&mut self, path: &str) -> Result<Arc<Sprite>, LoadError> {
        match self.cache.get(path) {
            Some(sprite) => Ok(sprite.clone()),
            None => self.load(path),
        }
    }
}
