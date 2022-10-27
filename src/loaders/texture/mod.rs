use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use derive_new::new;
use image::io::Reader as ImageReader;
use image::{EncodableLayout, ImageFormat, Rgba};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, ImmutableImage, MipmapsCount};
use vulkano::sync::{now, GpuFuture};

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::Texture;
use crate::loaders::GameFileLoader;

#[derive(new)]
pub struct TextureLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    #[new(value = "HashMap::new()")]
    cache: HashMap<String, Texture>,
}

impl TextureLoader {
    fn load(
        &mut self,
        path: &str,
        game_file_loader: &mut GameFileLoader,
        texture_future: &mut Box<dyn GpuFuture + 'static>,
    ) -> Result<Texture, String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture from {}{}{}", MAGENTA, path, NONE));

        let image_format = match &path[path.len() - 4..] {
            ".png" => ImageFormat::Png,
            ".bmp" | ".BMP" => ImageFormat::Bmp,
            ".tga" | ".TGA" => ImageFormat::Tga,
            extension => return Err(format!("unsupported file format {}", extension)),
        };

        let file_data = game_file_loader.get(&format!("data\\texture\\{}", path))?;
        let reader = ImageReader::with_format(Cursor::new(file_data), image_format);
        let mut image_buffer = reader
            .decode()
            .map_err(|error| format!("failed to decode image file ({})", error))?
            .to_rgba8();

        if image_format == ImageFormat::Bmp {
            image_buffer
                .pixels_mut()
                .filter(|pixel| pixel.0[0] == 255 && pixel.0[1] == 0 && pixel.0[2] == 255)
                .for_each(|pixel| *pixel = Rgba([0; 4]));
        }

        let image_data = image_buffer.as_bytes().to_vec();
        let dimensions = ImageDimensions::Dim2d {
            width: image_buffer.width(),
            height: image_buffer.height(),
            array_layers: 1,
        };

        let (image, future) = ImmutableImage::from_iter(
            image_data.iter().cloned(),
            dimensions,
            MipmapsCount::Log2,
            Format::R8G8B8A8_SRGB,
            self.queue.clone(),
        )
        .unwrap();

        let inner_future = std::mem::replace(texture_future, now(self.device.clone()).boxed());
        let combined_future = inner_future.join(future).boxed();
        *texture_future = combined_future;

        let texture = ImageView::new(Arc::new(image)).unwrap();
        self.cache.insert(path.to_string(), texture.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(texture)
    }

    pub fn get(
        &mut self,
        path: &str,
        game_file_loader: &mut GameFileLoader,
        texture_future: &mut Box<dyn GpuFuture + 'static>,
    ) -> Result<Texture, String> {
        match self.cache.get(path) {
            Some(texture) => Ok(texture.clone()),
            None => self.load(path, game_file_loader, texture_future),
        }
    }
}
