use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use derive_new::new;
use image::io::Reader as ImageReader;
use image::{EncodableLayout, ImageFormat, Rgba};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract};
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, ImmutableImage, MipmapsCount};
use vulkano::sync::{FenceSignalFuture, GpuFuture};

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{MemoryAllocator, Texture};
use crate::loaders::GameFileLoader;

#[derive(new)]
pub struct TextureLoader {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    #[new(default)]
    load_buffer: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer, MemoryAllocator>>,
    #[new(value = "HashMap::new()")]
    cache: HashMap<String, Texture>,
}

impl TextureLoader {
    fn load(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Texture, String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load texture from {}{}{}", MAGENTA, path, NONE));

        let image_format = match &path[path.len() - 4..] {
            ".png" => ImageFormat::Png,
            ".bmp" | ".BMP" => ImageFormat::Bmp,
            ".tga" | ".TGA" => ImageFormat::Tga,
            extension => return Err(format!("unsupported file format {}", extension)),
        };

        let file_data = game_file_loader.get(&format!("data\\texture\\{}", path))?;
        let reader = ImageReader::with_format(Cursor::new(file_data), image_format);
        let mut image_buffer = reader
            .decode()
            .map_err(|error| format!("failed to decode image file ({})", error))?
            .to_rgba8();

        if image_format == ImageFormat::Bmp {
            // These numbers are taken from https://github.com/Duckwhale/RagnarokFileFormats
            image_buffer
                .pixels_mut()
                .filter(|pixel| pixel.0[0] > 0xf0 && pixel.0[1] < 0x10 && pixel.0[2] > 0x0f)
                .for_each(|pixel| *pixel = Rgba([0; 4]));
        }

        let image_data = image_buffer.as_bytes().to_vec();
        let dimensions = ImageDimensions::Dim2d {
            width: image_buffer.width(),
            height: image_buffer.height(),
            array_layers: 1,
        };

        let load_buffer = self.load_buffer.get_or_insert_with(|| {
            AutoCommandBufferBuilder::primary(
                &*self.memory_allocator,
                self.queue.queue_family_index(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap()
        });

        let image = ImmutableImage::from_iter(
            &*self.memory_allocator,
            image_data.iter().cloned(),
            dimensions,
            MipmapsCount::Log2,
            Format::R8G8B8A8_SRGB,
            load_buffer,
        )
        .unwrap();

        let texture = ImageView::new_default(Arc::new(image)).unwrap();
        self.cache.insert(path.to_string(), texture.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(texture)
    }

    pub fn get(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Texture, String> {
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
