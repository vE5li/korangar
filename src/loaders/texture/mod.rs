use derive_new::new;
use std::collections::HashMap;
use std::sync::Arc;
use std::fs::File;
use std::io::{ Cursor, Read, BufReader };
use vulkano::device::{ Device, Queue };
use vulkano::image::{ ImageDimensions, ImmutableImage, MipmapsCount };
use vulkano::image::view::ImageView;
use vulkano::format::Format;
use vulkano::sync::{ GpuFuture, now };
use png::Decoder;
use bmp::open;

use graphics::Texture;

#[cfg(feature = "debug")]
use debug::*;

#[derive(new)]
pub struct TextureLoader {
    #[new(default)]
    cache: HashMap<String, Texture>,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl TextureLoader {

    fn load_png_data(path: &str) -> (Vec<u8>, ImageDimensions) {

        let file = File::open(path).expect("failed to open file");

        let mut reader = BufReader::new(file);
        let mut png_bytes = Vec::new();

        reader.read_to_end(&mut png_bytes).expect("failed to read texture data");

        let cursor = Cursor::new(png_bytes);
        let decoder = Decoder::new(cursor);
        let (info, mut reader) = decoder.read_info().unwrap();

        let dimensions = ImageDimensions::Dim2d {
            width: info.width,
            height: info.height,
            array_layers: 1,
        };

        let mut image_data = Vec::new();
        image_data.resize((info.width * info.height * 4) as usize, 0);
        reader.next_frame(&mut image_data).unwrap();

        return (image_data, dimensions);
    }

    fn load_bmp_data(path: &str) -> (Vec<u8>, ImageDimensions) {

        let image = open(&path).unwrap_or_else(|e| {
            panic!("Failed to open {}: {}", path, e); // return result ?
        });

        let mut image_data = Vec::new();

        for line in 0..image.get_height() {
            for column in 0..image.get_width() {
                let pixel = image.get_pixel(column, line);

                if pixel.r == 255 && pixel.g == 0 && pixel.b == 255 {
                    image_data.push(0);
                    image_data.push(0);
                    image_data.push(0);
                    image_data.push(0);
                } else {
                    image_data.push(pixel.r);
                    image_data.push(pixel.g);
                    image_data.push(pixel.b);
                    image_data.push(255);
                }
            }
        }

        let dimensions = ImageDimensions::Dim2d {
            width: image.get_width(),
            height: image.get_height(),
            array_layers: 1,
        };

        return (image_data, dimensions);
    }

    fn load(&mut self, path: String, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Texture {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture from {}{}{}", magenta(), path, none()));

        let (image_data, dimensions) = match &path[path.len() - 4..] {
            ".png" => Self::load_png_data(&path),
            ".bmp" | ".BMP" => Self::load_bmp_data(&path),
            extension => panic!("unsupported file format {}", extension),
        };

        let (image, future) = ImmutableImage::from_iter(image_data.iter().cloned(), dimensions, MipmapsCount::One, Format::R8G8B8A8_SRGB, self.queue.clone()).unwrap();

        let inner_future = std::mem::replace(texture_future, now(self.device.clone()).boxed());
        let combined_future = inner_future.join(future).boxed();
        *texture_future = combined_future;

        let texture = ImageView::new(image).unwrap();
        self.cache.insert(path, texture.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        return texture;
    }

    pub fn get(&mut self, path: String, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Texture {
        match self.cache.get(&path) {
            Some(texture) => return texture.clone(),
            None => return self.load(path, texture_future),
        }
    }
}
