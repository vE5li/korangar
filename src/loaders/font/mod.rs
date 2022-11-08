use std::sync::Arc;

use cgmath::{Array, Vector2};
use rusttype::gpu_cache::Cache;
use rusttype::*;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, BufferImageCopy, ClearColorImageInfo, CommandBufferUsage, CopyBufferToImageInfo, PrimaryCommandBufferAbstract,
};
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, ImageCreateFlags, ImageDimensions, ImageUsage, StorageImage};
use vulkano::sync::{FenceSignalFuture, GpuFuture};

use super::GameFileLoader;
use crate::graphics::{CommandBuilder, MemoryAllocator};

pub struct FontLoader {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    font_atlas: Arc<ImageView<StorageImage>>,
    cache: Box<Cache<'static>>,
    load_buffer: Option<CommandBuilder>,
    font: Box<Font<'static>>,
}

fn layout_paragraph(font: &Font<'static>, scale: Scale, width: u32, text: &str) -> Vec<PositionedGlyph<'static>> {
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    let mut caret = point(0.0, v_metrics.ascent);
    let mut last_glyph_id = None;

    for c in text.chars() {
        if c.is_control() {
            match c {
                '\r' => {
                    caret = point(0.0, caret.y + advance_height);
                }
                '\n' => {}
                _ => {}
            }
            continue;
        }

        let base_glyph = font.glyph(c);
        if let Some(id) = last_glyph_id.take() {
            caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }

        last_glyph_id = Some(base_glyph.id());
        let mut glyph = base_glyph.scaled(scale).positioned(caret);

        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > width as i32 {
                caret = point(0.0, caret.y + advance_height);
                glyph.set_position(caret);
                last_glyph_id = None;
            }
        }

        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push(glyph);
    }

    result
}

impl FontLoader {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, queue: Arc<Queue>, game_file_loader: &mut GameFileLoader) -> Self {
        let scale = 1.0; // get dynamically
        let cache_size = Vector2::from_value((256.0 * scale) as u32);
        let cache = Cache::builder().dimensions(cache_size.x, cache_size.y).build();

        let image_usage = ImageUsage {
            transfer_dst: true,
            sampled: true,
            ..ImageUsage::empty()
        };

        let image_dimensions = ImageDimensions::Dim2d {
            width: cache_size.x,
            height: cache_size.y,
            array_layers: 1,
        };

        // TODO: don't hardcode 2. This number is only used to determine the sharing
        // mode of the image. 1 = exclusive, 2 = concurrent
        let font_atlas_image = StorageImage::with_usage(
            &*memory_allocator,
            image_dimensions,
            Format::R8_UNORM, //R8G8B8A8_SRGB,
            image_usage,
            ImageCreateFlags::empty(),
            0..2,
        )
        .unwrap();

        let font_atlas = ImageView::new_default(font_atlas_image.clone()).unwrap();

        let font_path = "data\\WenQuanYiMicroHei.ttf";
        let data = game_file_loader.get(font_path).unwrap();
        let font = Font::try_from_vec(data).unwrap_or_else(|| {
            panic!("error constructing a Font from data at {:?}", font_path);
        });

        let mut builder = AutoCommandBufferBuilder::primary(
            &*memory_allocator,
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let clear_color_image_info = ClearColorImageInfo {
            clear_value: [0f32].into(),
            ..ClearColorImageInfo::image(font_atlas_image)
        };

        builder.clear_color_image(clear_color_image_info).unwrap();

        Self {
            memory_allocator,
            queue,
            font_atlas,
            cache: Box::new(cache),
            load_buffer: builder.into(),
            font: Box::new(font),
        }
    }

    pub fn get(&mut self, text: &str, font_size: f32) -> Vec<(Rect<f32>, Rect<i32>)> {
        let glyphs = layout_paragraph(&self.font, Scale::uniform(font_size), 500, text);

        for glyph in &glyphs {
            self.cache.queue_glyph(0, glyph.clone());
        }

        let buffer_usage = BufferUsage {
            transfer_src: true,
            ..BufferUsage::empty()
        };

        self.cache
            .cache_queued(|rect, data| {
                let builder = self.load_buffer.get_or_insert_with(|| {
                    AutoCommandBufferBuilder::primary(
                        &*self.memory_allocator,
                        self.queue.queue_family_index(),
                        CommandBufferUsage::OneTimeSubmit,
                    )
                    .unwrap()
                });

                let pixels = data.iter().map(|&value| value as i8);
                let buffer = CpuAccessibleBuffer::from_iter(&*self.memory_allocator, buffer_usage, false, pixels).unwrap();
                let image = self.font_atlas.image().clone();

                let region = BufferImageCopy {
                    image_subresource: image.subresource_layers(),
                    image_extent: [rect.width(), rect.height(), 1],
                    image_offset: [rect.min.x, rect.min.y, 0],
                    ..Default::default()
                };

                builder
                    .copy_buffer_to_image(CopyBufferToImageInfo {
                        regions: [region].into(),
                        ..CopyBufferToImageInfo::buffer_image(buffer, image)
                    })
                    .unwrap();
            })
            .unwrap();

        glyphs
            .into_iter()
            .filter_map(|glyph| self.cache.rect_for(0, &glyph).unwrap())
            .collect()
    }

    pub fn submit_load_buffer(&mut self) -> Option<FenceSignalFuture<Box<dyn GpuFuture>>> {
        self.load_buffer.take().map(|builder| {
            builder
                .build()
                .unwrap()
                .execute(self.queue.clone())
                .unwrap()
                .boxed()
                .then_signal_fence_and_flush()
                .unwrap()
        })
    }

    pub fn get_font_atlas(&self) -> Arc<ImageView<StorageImage>> {
        self.font_atlas.clone()
    }
}
