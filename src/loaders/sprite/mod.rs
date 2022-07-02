use derive_new::new;
use std::collections::HashMap;
use vulkano::sync::GpuFuture;
use std::rc::Rc;
use std::cell::RefCell;

#[cfg(feature = "debug")]
use debug::*;
use types::ByteStream;
use traits::ByteConvertable;
use loaders::GameFileLoader;
use types::Version;

#[derive(Clone)]
pub struct Sprite {} 

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
struct PaletteImage {
    pub width: u16,
    pub height: u16,
    #[length_hint(self.width * self.height)]
    pub encoded_data: EncodedData,
    //pub raw_data: Option<Vec<u8>>,
}

#[derive(Debug, ByteConvertable)]
struct RgbaImage {
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

#[derive(Debug, ByteConvertable)]
struct Palette {
    pub colors: [PaletteColor; 256],
}

#[derive(Debug, ByteConvertable)]
struct SpriteData {
    pub version: Version,
    pub palette_image_count: u16,
    pub rgba_image_count: u16,
    #[repeating(self.palette_image_count)]
    pub palette_images: Vec<PaletteImage>,
    #[repeating(self.rgba_image_count)]
    pub rgba_images: Vec<RgbaImage>,
//    #[version_equals_above(self.version, 1, 1)]
    pub palette: Palette,
}

#[derive(new)]
pub struct SpriteLoader {
    game_file_loader: Rc<RefCell<GameFileLoader>>,
    #[new(default)]
    cache: HashMap<String, Sprite>,
}

impl SpriteLoader {

    fn load(&mut self, path: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Sprite, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load sprite from {}{}{}", MAGENTA, path, NONE));

        let bytes = self.game_file_loader.borrow_mut().get(&format!("data\\sprite\\{}", path))?;
        let mut byte_stream = ByteStream::new(&bytes);
        
        if byte_stream.string(2).as_str() != "SP" {
            return Err(format!("failed to read magic number from {}", path));
        }

        let sprite_data = SpriteData::from_bytes(&mut byte_stream, None);

        let sprite = Sprite {};
        self.cache.insert(path.to_string(), sprite.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get(&mut self, path: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Sprite, String> {
        match self.cache.get(path) {
            Some(sprite) => Ok(sprite.clone()),
            None => self.load(path, texture_future),
        }
    }
}
