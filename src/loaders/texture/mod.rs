use derive_new::new;
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
use png::Decoder;
use bmp::from_reader;

#[cfg(feature = "debug")]
use debug::*;
use graphics::Texture;
use loaders::GameFileLoader;

#[derive(new)]
pub struct TextureLoader {
    game_file_loader: Rc<RefCell<GameFileLoader>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    #[new(value = "HashMap::new()")]
    cache: HashMap<String, Texture>,
}

impl TextureLoader {

    fn load_png_data(&self, path: &str) -> Result<(Vec<u8>, ImageDimensions), String> {

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

        Ok((image_data, dimensions))
    }

    fn load_bmp_data(&self, path: &str) -> Result<(Vec<u8>, ImageDimensions), String> {

        let bmp_bytes = self.game_file_loader.borrow_mut().get(&format!("data\\texture\\{}", path))?;
        let mut slice = bmp_bytes.as_slice();

        let image = from_reader(&mut slice).map_err(|error| error.to_string())?;

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

        Ok((image_data, dimensions))
    }

    fn load(&mut self, path: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Texture, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture from {}{}{}", MAGENTA, path, NONE));

        let (image_data, dimensions) = match &path[path.len() - 4..] {
            ".png" => self.load_png_data(&path),
            ".bmp" | ".BMP" => self.load_bmp_data(&path),
            extension => Err(format!("unsupported file format {}", extension)),
        }?;

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
