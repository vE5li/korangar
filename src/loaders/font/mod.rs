use std::sync::Arc;

use rusttype::*;
use rusttype::gpu_cache::Cache;
use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::command_buffer::{ AutoCommandBufferBuilder, PrimaryCommandBuffer, CommandBufferUsage };
use vulkano::image::{ AttachmentImage, ImageUsage };
use vulkano::image::view::ImageView;
use vulkano::format::Format;
use vulkano::device::{ Device, Queue };
use vulkano::sync::{ GpuFuture, now };

use crate::graphics::{ CommandBuilder, ImageBuffer };

pub struct FontLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    font_atlas: ImageBuffer,
    cache: Box<Cache<'static>>,
    builder: Option<CommandBuilder>,
    font: Box<Font<'static>>,
}

fn layout_paragraph(
    font: &Font<'static>,
    scale: Scale,
    width: u32,
    text: &str,
) -> Vec<PositionedGlyph<'static>> {
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

    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {

        let scale = 1.0; // get dynamically
        let cache_size = vector2!((512.0 * scale) as u32);
        let cache = Cache::builder()
            .dimensions(cache_size.x, cache_size.y)
            .build();

        let image_usage = ImageUsage {
            transfer_destination: true,
            sampled: true,
            ..ImageUsage::none()
        };

        let font_atlas_image = Arc::new(AttachmentImage::with_usage(device.clone(), cache_size.into(), Format::R8_SRGB, image_usage).unwrap());
        let font_atlas = ImageView::new(font_atlas_image.clone()).unwrap();

        let font_path = "raleway.medium.ttf";
        let data = std::fs::read(font_path).unwrap();
        let font = Font::try_from_vec(data).unwrap_or_else(|| {
            panic!("error constructing a Font from data at {:?}", font_path);
        });

        let mut builder = AutoCommandBufferBuilder::primary(device.clone(), queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();
        builder.clear_color_image(font_atlas_image, [0f32].into()).unwrap();

        Self { device, queue, font_atlas, cache: Box::new(cache), builder: builder.into(), font: Box::new(font) }
    }

    pub fn get(&mut self, text: &str) -> Vec<(PositionedGlyph, Rect<f32>)> {

        let scale = 1.0; // get dynamically
        let glyphs = layout_paragraph(&self.font, Scale::uniform(10.0 * scale), 500, &text);

        for glyph in &glyphs {
            self.cache.queue_glyph(0, glyph.clone());
        }

        let buffer_usage = BufferUsage{
            transfer_source: true,
            .. BufferUsage::none()
        };

        self.cache.cache_queued(|rect, data| {

            println!("{:?} ({} - {})", rect, rect.width(), rect.height());

            let builder = self.builder.get_or_insert_with(|| AutoCommandBufferBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap());

            let pixels: Vec<i8> = data
                .iter()
                .copied()
                .map(|value| value as i8)
                .collect();

            let buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), buffer_usage, false, pixels.into_iter()).unwrap();
            builder.copy_buffer_to_image_dimensions(buffer, self.font_atlas.image().clone(), [rect.min.x, rect.min.y, 0], [rect.width(), rect.height(), 1], 0, 1, 0).unwrap();

        }).unwrap();

        glyphs
            .into_iter()
            .filter_map(|glyph| self.cache.rect_for(0, &glyph).unwrap().map(|(floating, _integer)| (glyph, floating)))
            .collect()
    }

    pub fn flush(&mut self) -> Box<dyn GpuFuture> {

        let Some(builder) = self.builder.take() else {
            return now(self.device.clone()).boxed();
        };

        builder
            .build()
            .unwrap()
            .execute(self.queue.clone())
            .unwrap()
            .then_signal_semaphore_and_flush()
            .unwrap()
            .boxed()
    }

    pub fn get_font_atlas(&self) -> ImageBuffer {
        self.font_atlas.clone()
    }
}
