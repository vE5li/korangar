use std::collections::HashMap;
use std::sync::Arc;

use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_interface::elements::PrototypeElement;
use ragnarok_bytes::{ByteStream, FromBytes};
use ragnarok_formats::sprite::{PaletteColor, RgbaImageData, SpriteData};
use ragnarok_formats::version::InternalVersion;
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

use super::FALLBACK_SPRITE_FILE;
use crate::graphics::MemoryAllocator;
use crate::loaders::error::LoadError;
use crate::loaders::GameFileLoader;

#[derive(Clone, Debug, PrototypeElement)]
pub struct Sprite {
    #[hidden_element]
    pub textures: Vec<Arc<ImageView>>,
    #[cfg(feature = "debug")]
    sprite_data: SpriteData,
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
    fn load(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Sprite>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load sprite from {}", path.magenta()));

        let bytes = game_file_loader.get(&format!("data\\sprite\\{path}")).map_err(LoadError::File)?;
        let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

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

        // TODO: Move this to an extension trait in `korangar_loaders`.
        pub fn color_bytes(palette: &PaletteColor, index: u8) -> [u8; 4] {
            let alpha = match index {
                0 => 0,
                _ => 255,
            };

            [palette.red, palette.green, palette.blue, alpha]
        }

        let palette_images = sprite_data.palette_image_data.into_iter().map(|image_data| {
            // decode palette image data if necessary
            let data: Vec<u8> = image_data
                .data
                .0
                .iter()
                .flat_map(|palette_index| color_bytes(&palette.colors[*palette_index as usize], *palette_index))
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

    pub fn get(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Sprite>, LoadError> {
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
