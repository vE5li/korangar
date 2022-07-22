use cgmath::Vector2;
use derive_new::new;
use std::collections::HashMap;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use vulkano::device::{ Device, Queue };
use vulkano::image::{ ImageDimensions, ImmutableImage, MipmapsCount };
use vulkano::image::view::ImageView;
use vulkano::format::Format;
use vulkano::sync::{ GpuFuture, now };

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::types::ByteStream;
use crate::traits::ByteConvertable;
use crate::loaders::GameFileLoader;
use crate::types::Version;
use crate::graphics::{Texture, Renderer};

#[derive(Clone, PrototypeElement)]
pub struct Sprite {
    pub textures: Vec<Texture>,
}

//impl Sprite {
//
//    pub fn render(renderer: &mut Renderer, motion: &Motion, position: Vector2<usize>, mirror: bool, ext: bool, , opacity: u8) {
//    }
//}

#[derive(Debug)]
struct EncodedData(pub Vec<u8>);

impl ByteConvertable for EncodedData {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {

        let image_size = length_hint.unwrap();

        if image_size == 0 {
            return Self(Vec::new());
        }

        let mut data = vec![0; image_size];
        let mut encoded = u16::from_bytes(byte_stream, None);
        let mut next = 0;

        while next < image_size && encoded > 0 {

            let byte = byte_stream.next();
            encoded -= 1;

            if byte == 0 {

                let length = usize::max(byte_stream.next() as usize, 1);
                encoded -= 1;

                if next + length > image_size {
                    panic!("too much data encoded in palette image");
                }

                next += length;

            } else {
                data[next] = byte;
                next += 1;
            }
        }

        if next != image_size || encoded > 0 {
            panic!("badly encoded palette image");
        }

        Self(data)
    }
}

#[derive(Debug, ByteConvertable)]
struct PaletteImageData {
    pub width: u16,
    pub height: u16,
    #[version_equals_or_above(2, 1)]
    #[length_hint(self.width * self.height)]
    pub encoded_data: Option<EncodedData>,
    #[version_smaller(2, 1)]
    #[length_hint(self.width * self.height)]
    pub raw_data: Option<Vec<u8>>,
}

#[derive(Debug, ByteConvertable)]
struct RgbaImageData {
    pub width: u16,
    pub height: u16,
    #[length_hint(self.width * self.height)]
    pub data: Vec<u8>,
}

#[derive(Copy, Clone, Debug, Default, ByteConvertable)]
struct PaletteColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub reserved: u8,
}

impl PaletteColor {

    pub fn to_data(&self, index: u8) -> u32 {

        let alpha = match index {
            0 => 0,
            _other => 255,
        };

        (self.red as u32) | ((self.green as u32) << 8) | ((self.blue as u32) << 16) | (alpha << 24)
    }
}

#[derive(Debug, ByteConvertable)]
struct Palette {
    pub colors: [PaletteColor; 256],
}

#[derive(Debug, ByteConvertable)]
struct SpriteData {
    #[version]
    pub version: Version,
    pub palette_image_count: u16,
    #[version_equals_or_above(2, 0)]
    pub rgba_image_count: Option<u16>,
    #[repeating(self.palette_image_count)]
    pub palette_image_data: Vec<PaletteImageData>,
    #[repeating(self.rgba_image_count.unwrap_or_default())]
    pub rgba_image_data: Vec<RgbaImageData>,
    #[version_equals_or_above(1, 1)]
    pub palette: Option<Palette>,
}

#[derive(new)]
pub struct SpriteLoader {
    game_file_loader: Rc<RefCell<GameFileLoader>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    #[new(default)]
    cache: HashMap<String, Arc<Sprite>>,
}

impl SpriteLoader {

    fn load(&mut self, path: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Arc<Sprite>, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load sprite from {}{}{}", MAGENTA, path, NONE));

        let bytes = self.game_file_loader.borrow_mut().get(&format!("data\\sprite\\{}", path))?;
        let mut byte_stream = ByteStream::new(&bytes);

        if byte_stream.string(2).as_str() != "SP" {
            return Err(format!("failed to read magic number from {}", path));
        }

        let sprite_data = SpriteData::from_bytes(&mut byte_stream, None);

        assert!(byte_stream.is_empty());

        let rgba_images: Vec<Arc<ImmutableImage>> = sprite_data.rgba_image_data
            .into_iter()
            .map(|image_data| {
                let (image, future) = ImmutableImage::from_iter(image_data.data.iter().cloned(), ImageDimensions::Dim2d { width: image_data.width as u32, height: image_data.height as u32, array_layers: 1 }, MipmapsCount::One, Format::R8G8B8A8_SRGB, self.queue.clone()).unwrap();
                let inner_future = std::mem::replace(texture_future, now(self.device.clone()).boxed());
                let combined_future = inner_future.join(future).boxed();
                *texture_future = combined_future;
                image
            })
            .collect();

        let palette = sprite_data.palette.unwrap(); // unwrap_or_default() as soon as i know what
                                                    // the default palette is

        let palette_images: Vec<Arc<ImmutableImage>> = sprite_data.palette_image_data
            .into_iter()
            .map(|image_data| {

                let data: Vec<u32> = image_data.encoded_data
                    .map(|encoded| encoded.0)
                    .unwrap_or_else(|| image_data.raw_data.unwrap())
                    .iter()
                    .map(|palette_index| palette.colors[*palette_index as usize].to_data(*palette_index))
                    .collect();

                let (image, future) = ImmutableImage::from_iter(data.into_iter(), ImageDimensions::Dim2d { width: image_data.width as u32, height: image_data.height as u32, array_layers: 1 }, MipmapsCount::One, Format::R8G8B8A8_SRGB, self.queue.clone()).unwrap();
                let inner_future = std::mem::replace(texture_future, now(self.device.clone()).boxed());
                let combined_future = inner_future.join(future).boxed();
                *texture_future = combined_future;
                image
            })
            .collect();

        let textures = rgba_images
            .into_iter()
            .chain(palette_images.into_iter())
            .map(|image| ImageView::new(Arc::new(image)).unwrap())
            .collect();

        let sprite = Arc::new(Sprite { textures });
        self.cache.insert(path.to_string(), sprite.clone());

        println!("images: {}", sprite.textures.len());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get(&mut self, path: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Arc<Sprite>, String> {
        match self.cache.get(path) {
            Some(sprite) => Ok(sprite.clone()),
            None => self.load(path, texture_future),
        }
    }
}
