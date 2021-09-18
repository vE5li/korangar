use std::collections::HashMap;
use std::sync::Arc;

use vulkano::device::{ Device, Queue };
use vulkano::image::{ ImageDimensions, ImmutableImage, MipmapsCount };
use vulkano::image::view::ImageView;
use vulkano::format::Format;
use vulkano::sync::{ GpuFuture, now };

use bmp::open;

use graphics::Texture;

#[cfg(feature = "debug")]
use debug::*;

pub struct TextureManager {
    cache: HashMap<String, Texture>,
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl TextureManager {

    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        return Self {
            cache: HashMap::new(),
            device: device,
            queue: queue,
        }
    }

    fn load(&mut self, path: String) -> (Texture, Box<dyn GpuFuture + 'static>) {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture from {}{}{}", magenta(), path, none()));

        let image = open(&path).unwrap_or_else(|e| {
            panic!("Failed to open {}: {}", path, e); // return result ?
        });

        let mut image_data = Vec::new();

        for column in 0..image.get_width() {
            for line in 0..image.get_height() {
                let pixel = image.get_pixel(line, column);

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

        let (image, future) = ImmutableImage::from_iter(image_data.iter().cloned(), dimensions, MipmapsCount::One, Format::R8G8B8A8_SRGB, self.queue.clone()).unwrap();
        let texture = ImageView::new(image).unwrap();
        self.cache.insert(path, texture.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        return (texture, future.boxed());
    }

    pub fn get(&mut self, path: String) -> (Texture, Box<dyn GpuFuture + 'static>) {
        match self.cache.get(&path) {
            Some(texture) => return (texture.clone(), now(self.device.clone()).boxed()),
            None => return self.load(path),
        }
    }
}
