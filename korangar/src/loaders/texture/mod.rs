use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use derive_new::new;
use image::io::Reader as ImageReader;
use image::{EncodableLayout, ImageFormat, Rgba};
#[cfg(feature = "debug")]
use korangar_debug::Colorize;
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

use super::{FALLBACK_BMP_FILE, FALLBACK_PNG_FILE, FALLBACK_TGA_FILE};
use crate::graphics::MemoryAllocator;
use crate::loaders::GameFileLoader;

#[derive(new)]
pub struct TextureLoader {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    #[new(default)]
    load_buffer: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<MemoryAllocator>, MemoryAllocator>>,
    #[new(value = "HashMap::new()")]
    cache: HashMap<String, Arc<ImageView>>,
}

impl TextureLoader {
    fn load(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<ImageView>, String> {
        #[cfg(feature = "debug")]
        let timer = korangar_debug::Timer::new_dynamic(format!("load texture from {}", path.magenta()));

        let image_format = match &path[path.len() - 4..] {
            ".png" => ImageFormat::Png,
            ".bmp" | ".BMP" => ImageFormat::Bmp,
            ".tga" | ".TGA" => ImageFormat::Tga,
            extension => return Err(format!("unsupported file format {extension}")),
        };

        let file_data = game_file_loader.get(&format!("data\\texture\\{path}"))?;
        let reader = ImageReader::with_format(Cursor::new(file_data), image_format);

        let mut image_buffer = match reader.decode() {
            Ok(image) => image.to_rgba8(),
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    korangar_debug::print_debug!("Failed to decode image: {:?}", _error);
                    korangar_debug::print_debug!("Replacing with fallback");
                }

                let fallback_path = match image_format {
                    ImageFormat::Png => FALLBACK_PNG_FILE,
                    ImageFormat::Bmp => FALLBACK_BMP_FILE,
                    ImageFormat::Tga => FALLBACK_TGA_FILE,
                    _ => unreachable!(),
                };

                return self.get(fallback_path, game_file_loader);
            }
        };

        if image_format == ImageFormat::Bmp {
            // These numbers are taken from https://github.com/Duckwhale/RagnarokFileFormats
            image_buffer
                .pixels_mut()
                .filter(|pixel| pixel.0[0] > 0xF0 && pixel.0[1] < 0x10 && pixel.0[2] > 0x0F)
                .for_each(|pixel| *pixel = Rgba([0; 4]));
        }

        let load_buffer = self.load_buffer.get_or_insert_with(|| {
            AutoCommandBufferBuilder::primary(
                &*self.memory_allocator,
                self.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap()
        });

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
            image_buffer.as_bytes().iter().copied(),
        )
        .unwrap();

        let image = Image::new(
            &*self.memory_allocator,
            ImageCreateInfo {
                format: Format::R8G8B8A8_UNORM,
                extent: [image_buffer.width(), image_buffer.height(), 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();

        load_buffer
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(buffer, image.clone()))
            .unwrap();

        let texture = ImageView::new_default(image).unwrap();
        self.cache.insert(path.to_string(), texture.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(texture)
    }

    pub fn get(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<ImageView>, String> {
        match self.cache.get(path) {
            Some(texture) => Ok(texture.clone()),
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
