use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::io::Read;
use std::io::BufReader;
use std::fs::File;

use vulkano::device::{ Device, Queue };
use vulkano::image::{ ImageDimensions, ImmutableImage, MipmapsCount };
use vulkano::image::view::ImageView;
use vulkano::format::Format;
use vulkano::sync::{ GpuFuture, now };

use png::Decoder;

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

        let file = File::open(&path).expect("failed to open file");
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

        let (image, future) = ImmutableImage::from_iter(image_data.iter().cloned(), dimensions, MipmapsCount::One, Format::R8G8B8A8Srgb, self.queue.clone()).unwrap();
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
