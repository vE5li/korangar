use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use image::RgbaImage;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_interface::element::StateElement;
use korangar_util::FileLoader;
use korangar_util::color::premultiply_alpha;
use korangar_util::container::{Cacheable, SimpleCache};
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::sprite::{PaletteColor, RgbaImageData, SpriteData};
use ragnarok_formats::version::InternalVersion;
use rust_state::RustState;

use super::{FALLBACK_SPRITE_FILE, TextureLoader};
use crate::graphics::Texture;
use crate::loaders::GameFileLoader;
use crate::loaders::error::LoadError;

const MAX_CACHE_COUNT: u32 = 1000;
const MAX_CACHE_SIZE: usize = 512 << 20;

#[derive(Clone, Debug, RustState, StateElement)]
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
    game_file_loader: Arc<GameFileLoader>,
    texture_loader: Arc<TextureLoader>,
    cache: Mutex<SimpleCache<String, Arc<Sprite>>>,
}

impl SpriteLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>, texture_loader: Arc<TextureLoader>) -> Self {
        Self {
            game_file_loader,
            texture_loader,
            cache: Mutex::new(SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            )),
        }
    }

    fn load(&self, path: &str) -> Result<Arc<Sprite>, LoadError> {
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

                return self.get_or_load(FALLBACK_SPRITE_FILE);
            }
        };
        let mut byte_reader: ByteReader<Option<InternalVersion>> = ByteReader::with_default_metadata(&bytes);

        let sprite_data = match SpriteData::from_bytes(&mut byte_reader) {
            Ok(sprite_data) => sprite_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load sprite: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.get_or_load(FALLBACK_SPRITE_FILE);
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
            .map(|mut image_data| {
                premultiply_alpha(&mut image_data.data);

                self.texture_loader.create_color(
                    path,
                    RgbaImage::from_raw(image_data.width as u32, image_data.height as u32, image_data.data).unwrap(),
                    false,
                )
            })
            .collect();

        let sprite = Arc::new(Sprite {
            palette_size,
            textures,
            #[cfg(feature = "debug")]
            sprite_data: cloned_sprite_data,
        });

        let _result = self.cache.lock().unwrap().insert(path.to_string(), sprite.clone());

        #[cfg(feature = "debug")]
        if let Err(error) = _result {
            print_debug!(
                "[{}] sprite could not be added to cache. Path: '{}': {:?}",
                "error".red(),
                &path,
                error
            );
        }

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get_or_load(&self, path: &str) -> Result<Arc<Sprite>, LoadError> {
        let Some(sprite) = self.cache.lock().unwrap().get(path).cloned() else {
            return self.load(path);
        };

        Ok(sprite)
    }
}
