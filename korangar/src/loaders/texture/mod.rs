use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use derive_new::new;
use image::{EncodableLayout, ImageFormat, ImageReader, Rgba};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use wgpu::{Device, Extent3d, Queue, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use super::error::LoadError;
use super::{FALLBACK_BMP_FILE, FALLBACK_PNG_FILE, FALLBACK_TGA_FILE};
use crate::graphics::Texture;
use crate::loaders::GameFileLoader;

#[derive(new)]
pub struct TextureLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    #[new(default)]
    cache: HashMap<String, Arc<Texture>>,
}

impl TextureLoader {
    fn load(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Texture>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture from {}", path.magenta()));

        let image_format = match &path[path.len() - 4..] {
            ".png" => ImageFormat::Png,
            ".bmp" | ".BMP" => ImageFormat::Bmp,
            ".tga" | ".TGA" => ImageFormat::Tga,
            extension => return Err(LoadError::UnsupportedFormat(extension.to_owned())),
        };

        let file_data = game_file_loader.get(&format!("data\\texture\\{path}")).map_err(LoadError::File)?;
        let reader = ImageReader::with_format(Cursor::new(file_data), image_format);

        let mut image_buffer = match reader.decode() {
            Ok(image) => image.to_rgba8(),
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to decode image: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                let fallback_path = match image_format {
                    ImageFormat::Png => FALLBACK_PNG_FILE,
                    ImageFormat::Bmp => FALLBACK_BMP_FILE,
                    ImageFormat::Tga => FALLBACK_TGA_FILE,
                    _ => unreachable!(),
                };

                return self.get(fallback_path, game_file_loader);
            }
        };

        if image_format == ImageFormat::Bmp {
            // These numbers are taken from https://github.com/Duckwhale/RagnarokFileFormats
            image_buffer
                .pixels_mut()
                .filter(|pixel| pixel.0[0] > 0xF0 && pixel.0[1] < 0x10 && pixel.0[2] > 0x0F)
                .for_each(|pixel| *pixel = Rgba([0; 4]));
        }

        let texture = Texture::new_with_data(
            &self.device,
            &self.queue,
            &TextureDescriptor {
                label: Some(path),
                size: Extent3d {
                    width: image_buffer.width(),
                    height: image_buffer.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            image_buffer.as_bytes(),
        );
        let texture = Arc::new(texture);

        self.cache.insert(path.to_string(), texture.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(texture)
    }

    pub fn get(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Texture>, LoadError> {
        match self.cache.get(path) {
            Some(texture) => Ok(texture.clone()),
            None => self.load(path, game_file_loader),
        }
    }
}
