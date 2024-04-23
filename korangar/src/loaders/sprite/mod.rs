use std::collections::HashMap;
use std::sync::Arc;

use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_interface::elements::PrototypeElement;
use ragnarok_bytes::{ByteStream, ConversionError, ConversionResult, ConversionResultExt, FromBytes, FromBytesExt};
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract,
};
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::sync::future::FenceSignalFuture;
use vulkano::sync::GpuFuture;

use super::version::InternalVersion;
use super::FALLBACK_SPRITE_FILE;
use crate::graphics::MemoryAllocator;
use crate::loaders::{GameFileLoader, MinorFirst, Version};

#[derive(Clone, Debug, PrototypeElement)]
pub struct Sprite {
    #[hidden_element]
    pub textures: Vec<Arc<ImageView>>,
    #[cfg(feature = "debug")]
    sprite_data: SpriteData,
}

#[derive(Clone, Debug, PrototypeElement)]
struct PaletteImageData {
    pub width: u16,
    pub height: u16,
    pub data: EncodedData,
}

#[derive(Clone, Debug, PrototypeElement)]
struct EncodedData(pub Vec<u8>);

impl FromBytes for PaletteImageData {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self>
    where
        Self: Sized,
    {
        let width = u16::from_bytes(byte_stream).trace::<Self>()?;
        let height = u16::from_bytes(byte_stream).trace::<Self>()?;

        let data = match width as usize * height as usize {
            0 => Vec::new(),
            image_size
                if byte_stream
                    .get_metadata::<Self, Option<InternalVersion>>()?
                    .ok_or(ConversionError::from_message("version not set"))?
                    .smaller(2, 1) =>
            {
                Vec::from_n_bytes(byte_stream, image_size).trace::<Self>()?
            }
            image_size => {
                let mut data = vec![0; image_size];
                let mut encoded = u16::from_bytes(byte_stream).trace::<Self>()?;
                let mut next = 0;

                while next < image_size && encoded > 0 {
                    let byte = byte_stream.byte::<Self>()?;
                    encoded -= 1;

                    if byte == 0 {
                        let length = usize::max(byte_stream.byte::<Self>()? as usize, 1);
                        encoded -= 1;

                        if next + length > image_size {
                            return Err(ConversionError::from_message("too much data encoded in palette image"));
                        }

                        next += length;
                    } else {
                        data[next] = byte;
                        next += 1;
                    }
                }

                if next != image_size || encoded > 0 {
                    return Err(ConversionError::from_message("badly encoded palette image"));
                }

                data
            }
        };

        Ok(Self {
            width,
            height,
            data: EncodedData(data),
        })
    }
}

#[derive(Clone, Debug, FromBytes, PrototypeElement)]
struct RgbaImageData {
    pub width: u16,
    pub height: u16,
    #[length_hint(self.width as usize * self.height as usize * 4)]
    pub data: Vec<u8>,
}

#[derive(Copy, Clone, Debug, Default, FromBytes, PrototypeElement)]
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

#[derive(Clone, Debug, FromBytes, PrototypeElement)]
struct Palette {
    pub colors: [PaletteColor; 256],
}

#[derive(Clone, Debug, FromBytes, PrototypeElement)]
struct SpriteData {
    #[version]
    pub version: Version<MinorFirst>,
    pub palette_image_count: u16,
    #[version_equals_or_above(1, 2)]
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
    load_buffer: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<MemoryAllocator>, MemoryAllocator>>,
    #[new(default)]
    cache: HashMap<String, Arc<Sprite>>,
}

impl SpriteLoader {
    fn load(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Sprite>, String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load sprite from {}", path.magenta()));

        let bytes = game_file_loader.get(&format!("data\\sprite\\{path}"))?;
        let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

        if <[u8; 2]>::from_bytes(&mut byte_stream).unwrap() != [b'S', b'P'] {
            return Err(format!("failed to read magic number from {path}"));
        }

        let sprite_data = match SpriteData::from_bytes(&mut byte_stream) {
            Ok(sprite_data) => sprite_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load sprite: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.get(FALLBACK_SPRITE_FILE, game_file_loader);
            }
        };

        #[cfg(feature = "debug")]
        let cloned_sprite_data = sprite_data.clone();

        let palette = sprite_data.palette.unwrap(); // unwrap_or_default() as soon as i know what
        // the default palette is

        let rgba_images/*: Vec<Arc<ImmutableImage>>*/ = sprite_data
            .rgba_image_data
            .into_iter();

        let palette_images = sprite_data.palette_image_data.into_iter().map(|image_data| {
            // decode palette image data if necessary
            let data: Vec<u8> = image_data
                .data
                .0
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
                let buffer = Buffer::from_iter(
                    &*self.memory_allocator,
                    BufferCreateInfo {
                        usage: BufferUsage::TRANSFER_SRC,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_HOST | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                    image_data.data.iter().copied(),
                )
                .unwrap();

                let image = Image::new(
                    &*self.memory_allocator,
                    ImageCreateInfo {
                        format: Format::R8G8B8A8_UNORM,
                        extent: [image_data.width as u32, image_data.height as u32, 1],
                        usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default(),
                )
                .unwrap();

                load_buffer
                    .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(buffer, image.clone()))
                    .unwrap();

                ImageView::new_default(image).unwrap()
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
