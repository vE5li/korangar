use std::collections::HashMap;
use std::sync::Arc;

use derive_new::new;
use procedural::*;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, ImmutableImage, MipmapsCount};
use vulkano::sync::{now, FenceSignalFuture, GpuFuture};

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{MemoryAllocator, Texture};
use crate::interface::{ElementCell, PrototypeElement};
use crate::loaders::{ByteConvertable, ByteStream, GameFileLoader, Version};

#[derive(Clone, PrototypeElement)]
pub struct Sprite {
    #[hidden_element]
    pub textures: Vec<Texture>,
    #[cfg(feature = "debug")]
    sprite_data: SpriteData,
}

#[derive(Clone, Debug)]
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

impl PrototypeElement for EncodedData {
    fn to_element(&self, display: String) -> ElementCell {
        self.0.to_element(display)
    }
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
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

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct RgbaImageData {
    pub width: u16,
    pub height: u16,
    #[length_hint(self.width * self.height)]
    pub data: Vec<u8>,
}

#[derive(Copy, Clone, Debug, Default, ByteConvertable, PrototypeElement)]
struct PaletteColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub reserved: u8,
}

impl PaletteColor {
    pub fn color_bytes(&self, index: u8) -> [u8; 4] {
        let alpha = match index {
            0 => 0,
            _ => 255,
        };

        [self.red, self.green, self.blue, alpha]
    }
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct Palette {
    pub colors: [PaletteColor; 256],
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
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
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    #[new(default)]
    load_buffer: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer, MemoryAllocator>>,
    #[new(default)]
    cache: HashMap<String, Arc<Sprite>>,
}

impl SpriteLoader {
    fn load(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Sprite>, String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load sprite from {}{}{}", MAGENTA, path, NONE));

        let bytes = game_file_loader.get(&format!("data\\sprite\\{}", path))?;
        let mut byte_stream = ByteStream::new(&bytes);

        if byte_stream.string(2).as_str() != "SP" {
            return Err(format!("failed to read magic number from {}", path));
        }

        let sprite_data = SpriteData::from_bytes(&mut byte_stream, None);
        #[cfg(feature = "debug")]
        let cloned_sprite_data = sprite_data.clone();

        assert!(byte_stream.is_empty());

        let palette = sprite_data.palette.unwrap(); // unwrap_or_default() as soon as i know what
        // the default palette is

        let rgba_images/*: Vec<Arc<ImmutableImage>>*/ = sprite_data
            .rgba_image_data
            .into_iter();

        let palette_images = sprite_data.palette_image_data.into_iter().map(|image_data| {
            // decode palette image data if necessary
            let data: Vec<u8> = image_data
                .encoded_data
                .map(|encoded| encoded.0)
                .unwrap_or_else(|| image_data.raw_data.unwrap())
                .iter()
                .flat_map(|palette_index| palette.colors[*palette_index as usize].color_bytes(*palette_index))
                .collect();

            RgbaImageData {
                width: image_data.width,
                height: image_data.height,
                data,
            }
        });

        let load_buffer = self.load_buffer.get_or_insert_with(|| {
            AutoCommandBufferBuilder::primary(
                &*self.memory_allocator,
                self.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap()
        });

        let textures = rgba_images
            .chain(palette_images)
            .map(|image_data| {
                let image = ImmutableImage::from_iter(
                    &*self.memory_allocator,
                    image_data.data.iter().cloned(),
                    ImageDimensions::Dim2d {
                        width: image_data.width as u32,
                        height: image_data.height as u32,
                        array_layers: 1,
                    },
                    MipmapsCount::One,
                    Format::R8G8B8A8_SRGB,
                    load_buffer,
                )
                .unwrap();

                ImageView::new_default(Arc::new(image)).unwrap()
            })
            .collect();

        let sprite = Arc::new(Sprite {
            textures,
            #[cfg(feature = "debug")]
            sprite_data: cloned_sprite_data,
        });

        self.cache.insert(path.to_string(), sprite.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Sprite>, String> {
        match self.cache.get(path) {
            Some(sprite) => Ok(sprite.clone()),
            None => self.load(path, game_file_loader),
        }
    }

    pub fn submit_load_buffer(&mut self) -> Option<FenceSignalFuture<Box<dyn GpuFuture>>> {
        self.load_buffer.take().map(|buffer| {
            buffer
                .build()
                .unwrap()
                .execute(self.queue.clone())
                .unwrap()
                .boxed()
                .then_signal_fence_and_flush()
                .unwrap()
        })
    }
}
