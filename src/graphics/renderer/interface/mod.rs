use std::sync::Arc;
use vulkano::{device::Device, format::Format};
use vulkano::device::Queue;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Subpass, RenderPass};
use vulkano::image::ImageUsage;

use crate::{types::maths::*, graphics::ImageBuffer};

use super::{ DeferredRenderTarget, Renderer, Camera, GeometryRenderer as GeometryRendererTrait, EntityRenderer as EntityRendererTrait, SingleRenderTarget };
use crate::graphics::{ Texture, ModelVertexBuffer, WaterVertexBuffer, Color };

pub struct InterfaceRenderer { 
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    dimensions: [u32; 2],
}

impl InterfaceRenderer { 

    pub fn new(device: Arc<Device>, queue: Arc<Queue>, viewport: Viewport, dimensions: [u32; 2]) -> Self {

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                interface: {
                    load: Clear,
                    store: Store,
                    format: Format::R8G8B8A8_SRGB,
                    samples: 1,
                }
            },
            pass: {
                color: [interface],
                depth_stencil: {}
            }
        )
        .unwrap();

        Self {
            device,
            queue,
            render_pass,
            dimensions,
        }
    }

    pub fn recreate_pipeline(&mut self, viewport: Viewport, dimensions: [u32; 2]) {

        self.dimensions = dimensions;
    }

    pub fn create_render_target(&self) -> <Self as Renderer>::Target {

        let image_usage = ImageUsage {
            sampled: true,
            transfer_destination: true,
            color_attachment: true,
            input_attachment: true,
            ..ImageUsage::none()
        };

        <Self as Renderer>::Target::new(self.device.clone(), self.queue.clone(), self.render_pass.clone(), self.dimensions, image_usage, vulkano::format::ClearValue::Float([0.0, 0.0, 0.0, 1.0]))
    }

    pub fn render_sprite(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, texture: Texture, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color, smooth: bool) {
        //self.sprite_renderer.render(render_target, self.window_size, texture, position, size, clip_size, color, smooth);
    }

    pub fn render_sprite_indexed(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, texture: Texture, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color, column_count: usize, cell_index: usize, smooth: bool) {
        //self.sprite_renderer.render_indexed(render_target, self.window_size, texture, position, size, clip_size, color, column_count, cell_index, smooth);
    }

    pub fn render_rectangle(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, corner_radius: Vector4<f32>, color: Color) {
        //self.rectangle_renderer.render(render_target, self.window_size, position, size, clip_size, corner_radius, color);
    }

    pub fn render_checkbox(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color, checked: bool) {
        /*match checked {
            true => self.render_sprite(render_target, self.checked_box_texture.clone(), position, size, clip_size, color, true),
            false => self.render_sprite(render_target, self.unchecked_box_texture.clone(), position, size, clip_size, color, true),
        }*/
    }

    pub fn render_expand_arrow(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color, expanded: bool) {
        /*match expanded {
            true => self.render_sprite(render_target, self.expanded_arrow_texture.clone(), position, size, clip_size, color, true),
            false => self.render_sprite(render_target, self.collapsed_arrow_texture.clone(), position, size, clip_size, color, true),
        }*/
    }

    pub fn render_text(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, text: &str, position: Vector2<f32>, clip_size: Vector2<f32>, color: Color, font_size: f32) {
        /*for character in text.as_bytes() {
            let index = (*character as usize).saturating_sub(31);
            self.render_sprite_indexed(render_target, self.font_map.clone(), position, Vector2::new(font_size, font_size), clip_size, color, 10, index, true);
            position.x += font_size / 2.0;
        }*/
    }

    /*pub fn render_text_new(&self, text: &str, position: Vector2<f32>, clip_size: Vector2<f32>, color: Color, font_size: f32) {
        self.text_renderer.render(&mut current_frame.builder, self.window_size, position, vector2!(font_size), clip_size, color);
    }*/

    #[cfg(feature = "debug")]
    pub fn render_debug_icon(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color) {
        //self.render_sprite(self.debug_icon_texture.clone(), position, size, clip_size, color, true);
    }
}

impl Renderer for InterfaceRenderer {
    type Target = SingleRenderTarget<{ Format::R8G8B8A8_SRGB }>;
}
