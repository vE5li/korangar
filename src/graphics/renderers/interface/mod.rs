mod rectangle;
mod sprite;
mod text;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use cgmath::{Vector2, Vector4};
use vulkano::device::{Device, Queue};
use vulkano::format::{ClearColorValue, Format};
use vulkano::image::{ImageUsage, SampleCount};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::RenderPass;
use vulkano::sync::{now, GpuFuture};

use self::rectangle::RectangleRenderer;
use self::sprite::SpriteRenderer;
use self::text::TextRenderer;
use crate::graphics::{Color, Renderer, SingleRenderTarget, Texture};
use crate::loaders::{FontLoader, GameFileLoader, TextureLoader};

pub struct InterfaceRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    rectangle_renderer: RectangleRenderer,
    sprite_renderer: SpriteRenderer,
    text_renderer: TextRenderer,
    font_map: Texture,
    checked_box_texture: Texture,
    unchecked_box_texture: Texture,
    expanded_arrow_texture: Texture,
    collapsed_arrow_texture: Texture,
    dimensions: [u32; 2],
}

impl InterfaceRenderer {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        viewport: Viewport,
        dimensions: [u32; 2],
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        font_loader: Rc<RefCell<FontLoader>>,
    ) -> Self {
        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                interface: {
                    load: DontCare,
                    store: Store,
                    format: Format::R8G8B8A8_SRGB,
                    samples: 4,
                }
            },
            pass: {
                color: [interface],
                depth_stencil: {}
            }
        )
        .unwrap();

        let subpass = render_pass.clone().first_subpass();
        let rectangle_renderer = RectangleRenderer::new(device.clone(), subpass.clone(), viewport.clone());
        let sprite_renderer = SpriteRenderer::new(device.clone(), subpass.clone(), viewport.clone());
        let font_renderer = TextRenderer::new(device.clone(), subpass, viewport, font_loader);

        let mut texture_future = now(device.clone()).boxed();
        let font_map = texture_loader.get("font.png", game_file_loader, &mut texture_future).unwrap();
        let checked_box_texture = texture_loader
            .get("checked_box.png", game_file_loader, &mut texture_future)
            .unwrap();
        let unchecked_box_texture = texture_loader
            .get("unchecked_box.png", game_file_loader, &mut texture_future)
            .unwrap();
        let expanded_arrow_texture = texture_loader
            .get("expanded_arrow.png", game_file_loader, &mut texture_future)
            .unwrap();
        let collapsed_arrow_texture = texture_loader
            .get("collapsed_arrow.png", game_file_loader, &mut texture_future)
            .unwrap();

        texture_future.flush().unwrap();
        texture_future.cleanup_finished();

        Self {
            device,
            queue,
            render_pass,
            rectangle_renderer,
            sprite_renderer,
            text_renderer: font_renderer,
            font_map,
            checked_box_texture,
            unchecked_box_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,
            dimensions,
        }
    }

    pub fn recreate_pipeline(&mut self, viewport: Viewport, dimensions: [u32; 2]) {
        let subpass = self.render_pass.clone().first_subpass();

        self.rectangle_renderer
            .recreate_pipeline(self.device.clone(), subpass.clone(), viewport.clone());
        self.sprite_renderer.recreate_pipeline(self.device.clone(), subpass, viewport);
        self.dimensions = dimensions;
    }

    pub fn create_render_target(&self) -> <Self as Renderer>::Target {
        let image_usage = ImageUsage {
            sampled: true,
            transfer_dst: true,
            color_attachment: true,
            input_attachment: true,
            ..ImageUsage::empty()
        };

        <Self as Renderer>::Target::new(
            self.device.clone(),
            self.queue.clone(),
            self.render_pass.clone(),
            self.dimensions,
            SampleCount::Sample4,
            image_usage,
            ClearColorValue::Float([0.0, 0.0, 0.0, 0.0]),
        )
    }

    pub fn render_sprite(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        texture: Texture,
        position: Vector2<f32>,
        size: Vector2<f32>,
        clip_size: Vector4<f32>,
        color: Color,
        smooth: bool,
    ) {
        let window_size = Vector2::new(self.dimensions[0] as usize, self.dimensions[1] as usize);
        self.sprite_renderer
            .render(render_target, texture, window_size, position, size, clip_size, color, smooth);
    }

    pub fn render_sprite_indexed(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        texture: Texture,
        position: Vector2<f32>,
        size: Vector2<f32>,
        clip_size: Vector4<f32>,
        color: Color,
        column_count: usize,
        cell_index: usize,
        smooth: bool,
    ) {
        let window_size = Vector2::new(self.dimensions[0] as usize, self.dimensions[1] as usize);
        self.sprite_renderer.render_indexed(
            render_target,
            texture,
            window_size,
            position,
            size,
            clip_size,
            color,
            column_count,
            cell_index,
            smooth,
        );
    }

    pub fn render_rectangle(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        position: Vector2<f32>,
        size: Vector2<f32>,
        clip_size: Vector4<f32>,
        corner_radius: Vector4<f32>,
        color: Color,
    ) {
        let window_size = Vector2::new(self.dimensions[0] as usize, self.dimensions[1] as usize);
        self.rectangle_renderer
            .render(render_target, window_size, position, size, clip_size, corner_radius, color);
    }

    pub fn render_checkbox(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        position: Vector2<f32>,
        size: Vector2<f32>,
        clip_size: Vector4<f32>,
        color: Color,
        checked: bool,
    ) {
        let texture = match checked {
            true => self.checked_box_texture.clone(),
            false => self.unchecked_box_texture.clone(),
        };

        self.render_sprite(render_target, texture, position, size, clip_size, color, true);
    }

    pub fn render_expand_arrow(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        position: Vector2<f32>,
        size: Vector2<f32>,
        clip_size: Vector4<f32>,
        color: Color,
        expanded: bool,
    ) {
        let texture = match expanded {
            true => self.expanded_arrow_texture.clone(),
            false => self.collapsed_arrow_texture.clone(),
        };

        self.render_sprite(render_target, texture, position, size, clip_size, color, true);
    }

    pub fn render_text(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        text: &str,
        mut position: Vector2<f32>,
        clip_size: Vector4<f32>,
        color: Color,
        font_size: f32,
    ) {
        for character in text.as_bytes() {
            let index = (*character as usize).saturating_sub(31);
            self.render_sprite_indexed(
                render_target,
                self.font_map.clone(),
                position,
                Vector2::new(font_size, font_size),
                clip_size,
                color,
                10,
                index,
                true,
            );
            position.x += font_size / 2.0;
        }
    }

    pub fn render_text_new(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        text: &str,
        position: Vector2<f32>,
        clip_size: Vector4<f32>,
        color: Color,
        font_size: f32,
    ) {
        let window_size = Vector2::new(self.dimensions[0] as usize, self.dimensions[1] as usize);
        self.text_renderer.render(
            render_target,
            text,
            window_size,
            position,
            clip_size,
            color,
            font_size,
        );
    }
}

impl Renderer for InterfaceRenderer {
    type Target = SingleRenderTarget<{ Format::R8G8B8A8_SRGB }, (), ClearColorValue>;
}
