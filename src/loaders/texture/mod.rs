use derive_new::new;
use image::{EncodableLayout, Rgba, ImageFormat};
use std::collections::HashMap;
use std::sync::Arc;
use std::fs::File;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{ Cursor, Read, BufReader };
use vulkano::device::{ Device, Queue };
use vulkano::image::{ ImageDimensions, ImmutableImage, MipmapsCount };
use vulkano::image::view::ImageView;
use vulkano::format::Format;
use vulkano::sync::{ GpuFuture, now };
use image::io::Reader as ImageReader;

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::Texture;
use crate::loaders::GameFileLoader;

#[derive(new)]
pub struct TextureLoader {
    game_file_loader: Rc<RefCell<GameFileLoader>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    #[new(value = "HashMap::new()")]
    cache: HashMap<String, Texture>,
}

impl TextureLoader {

    fn load(&mut self, path: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Texture, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture from {}{}{}", MAGENTA, path, NONE));

        let image_format = match &path[path.len() - 4..] {
            ".png" => ImageFormat::Png,
            ".bmp" | ".BMP" => ImageFormat::Bmp,
            ".tga" | ".TGA" => ImageFormat::Tga,
            extension => return Err(format!("unsupported file format {}", extension)),
        };

        let file_data = if image_format == ImageFormat::Png {

            let file = File::open(path).expect("failed to open file");
            let mut reader = BufReader::new(file);
            let mut png_bytes = Vec::new();

            reader.read_to_end(&mut png_bytes).expect("failed to read texture data");
            png_bytes
        } else {
            self.game_file_loader.borrow_mut().get(&format!("data\\texture\\{}", path))?
        };

        let reader = ImageReader::with_format(Cursor::new(file_data), image_format);
        let mut image_buffer = reader.decode().map_err(|error| format!("failed to decode image file ({})", error))?.to_rgba8();

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

        let (image, future) = ImmutableImage::from_iter(image_data.iter().cloned(), dimensions, MipmapsCount::Log2, Format::R8G8B8A8_SRGB, self.queue.clone()).unwrap();

        let inner_future = std::mem::replace(texture_future, now(self.device.clone()).boxed());
        let combined_future = inner_future.join(future).boxed();
        *texture_future = combined_future;

        let texture = ImageView::new(Arc::new(image)).unwrap();
        self.cache.insert(path.to_string(), texture.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(texture)
    }

    pub fn get(&mut self, path: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Texture, String> {
        match self.cache.get(path) {
            Some(texture) => Ok(texture.clone()),
            None => self.load(path, texture_future),
        }
    }
}
