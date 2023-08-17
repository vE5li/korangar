use std::sync::Arc;

use cgmath::{Array, Vector2};
use rusttype::gpu_cache::Cache;
use rusttype::*;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, BufferImageCopy, ClearColorImageInfo, CommandBufferUsage, CopyBufferToImageInfo, PrimaryCommandBufferAbstract,
};
use vulkano::device::Queue;
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::sync::future::FenceSignalFuture;
use vulkano::sync::GpuFuture;

use super::GameFileLoader;
use crate::graphics::{Color, CommandBuilder, MemoryAllocator};

pub struct FontLoader {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    font_atlas: Arc<ImageView>,
    cache: Box<Cache<'static>>,
    load_buffer: Option<CommandBuilder>,
    font: Box<Font<'static>>,
}

struct GlyphData {
    glyph: PositionedGlyph<'static>,
    color: Color,
}

fn layout_paragraph(font: &Font<'static>, scale: Scale, width: f32, text: &str, default_color: Color) -> (Vec<GlyphData>, Vector2<f32>) {
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    let mut caret = point(0.0, v_metrics.ascent);
    let mut last_glyph_id = None;
    let mut color = default_color;
    let mut chars = text.chars();

    while let Some(character) = chars.next() {
        if character.is_control() {
            match character {
                '\r' => {
                    caret = point(0.0, caret.y + advance_height);
                }
                '\n' => {}
                _ => {}
            }
            continue;
        }

        // Color code following.
        if character == '^' {
            let mut cloned_chars = chars.clone();

            // If the next 6 characters are Hex digits (0-9 A-F)
            if (0..6)
                .map(|_| cloned_chars.next())
                .all(|option| option.is_some_and(|character| character.is_ascii_hexdigit()))
            {
                // Advance the actual iterator
                let color_bytes: [u8; 6] = chars.next_chunk().unwrap().map(|character| character as u8);

                // We made sure that all characters are ascii hexdigits, so this is completetly
                // safe.
                let color_code = unsafe { std::str::from_utf8_unchecked(&color_bytes) };

                color = match color_code {
                    "000000" => default_color,
                    code => Color::rgb_hex(code),
                };

                continue;
            }
        }

        let base_glyph = font.glyph(character);
        if let Some(id) = last_glyph_id.take() {
            caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }

        last_glyph_id = Some(base_glyph.id());
        let mut glyph = base_glyph.scaled(scale).positioned(caret);

        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x as f32 > width {
                caret = point(0.0, caret.y + advance_height);
                glyph.set_position(caret);
                last_glyph_id = None;
            }
        }

        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push(GlyphData { glyph, color });
    }

    (result, Vector2::new(caret.x, caret.y))
}

impl FontLoader {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, queue: Arc<Queue>, game_file_loader: &mut GameFileLoader) -> Self {
        let cache_size = Vector2::from_value(512);
        let cache = Cache::builder().dimensions(cache_size.x, cache_size.y).build();

        let font_atlas_image = Image::new(
            &*memory_allocator,
            ImageCreateInfo {
                format: Format::R8_UNORM,
                extent: [cache_size.x, cache_size.y, 1],
                usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();
        let font_atlas = ImageView::new_default(font_atlas_image.clone()).unwrap();

        let font_path = "data\\WenQuanYiMicroHei.ttf";
        let data = game_file_loader.get(font_path).unwrap();
        let font = Font::try_from_vec(data).unwrap_or_else(|| {
            panic!("error constructing a Font from data at {font_path:?}");
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

    pub fn get_text_dimensions(&self, text: &str, font_size: f32, available_width: f32) -> Vector2<f32> {
        let (_, size) = layout_paragraph(
            &self.font,
            Scale::uniform(font_size),
            available_width,
            text,
            Color::monochrome(0),
        );

        size
    }

    pub fn get(
        &mut self,
        text: &str,
        default_color: Color,
        font_size: f32,
        available_width: f32,
    ) -> (Vec<(Rect<f32>, Rect<i32>, Color)>, f32) {
        let (glyphs, size) = layout_paragraph(&self.font, Scale::uniform(font_size), available_width, text, default_color);

        for glyph in &glyphs {
            self.cache.queue_glyph(0, glyph.glyph.clone());
        }

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
                    pixels,
                )
                .unwrap();

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

        (
            glyphs
                .into_iter()
                .filter_map(|glyph| {
                    self.cache
                        .rect_for(0, &glyph.glyph)
                        .unwrap()
                        .map(|tuple| (tuple.0, tuple.1, glyph.color))
                })
                .collect(),
            size.y,
        )
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

    pub fn get_font_atlas(&self) -> Arc<ImageView> {
        self.font_atlas.clone()
    }
}
