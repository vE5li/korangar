mod rectangle;
mod sprite;
mod text;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use korangar_interface::application::Application;
use korangar_procedural::profile;
use vulkano::device::{DeviceOwned, Queue};
use vulkano::format::{ClearColorValue, Format};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageUsage, SampleCount};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::RenderPass;

use self::rectangle::RectangleRenderer;
use self::sprite::SpriteRenderer;
use self::text::TextRenderer;
use super::{IntoFormat, SubpassAttachments};
use crate::graphics::{Color, MemoryAllocator, Renderer, SingleRenderTarget, SpriteRenderer as SpriteRendererTrait};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{FontLoader, GameFileLoader, TextureLoader};

#[derive(PartialEq, Eq)]
pub enum InterfaceSubrenderer {
    Rectangle,
    Sprite,
    Text,
}

pub struct InterfaceRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    font_loader: Rc<RefCell<FontLoader>>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    rectangle_renderer: RectangleRenderer,
    sprite_renderer: SpriteRenderer,
    text_renderer: TextRenderer,
    checked_box_texture: Arc<ImageView>,
    unchecked_box_texture: Arc<ImageView>,
    expanded_arrow_texture: Arc<ImageView>,
    collapsed_arrow_texture: Arc<ImageView>,
    dimensions: [u32; 2],
}

impl InterfaceRenderer {
    const fn subpass() -> SubpassAttachments {
        SubpassAttachments { color: 1, depth: 0 }
    }

    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        font_loader: Rc<RefCell<FontLoader>>,
        queue: Arc<Queue>,
        viewport: Viewport,
        dimensions: [u32; 2],
    ) -> Self {
        let device = memory_allocator.device().clone();
        let render_pass = vulkano::single_pass_renderpass!(
            device,
            attachments: {
                interface: {
                    format: Format::R8G8B8A8_UNORM,
                    samples: 4,
                    load_op: DontCare,
                    store_op: Store,
                }
            },
            pass: {
                color: [interface],
                depth_stencil: {}
            }
        )
        .unwrap();

        let subpass = render_pass.clone().first_subpass();
        let rectangle_renderer = RectangleRenderer::new(memory_allocator.clone(), subpass.clone(), viewport.clone());
        let sprite_renderer = SpriteRenderer::new(memory_allocator.clone(), subpass.clone(), viewport.clone());
        let font_renderer = TextRenderer::new(memory_allocator.clone(), subpass, viewport, font_loader.clone());

        let checked_box_texture = texture_loader.get("checked_box.png", game_file_loader).unwrap();
        let unchecked_box_texture = texture_loader.get("unchecked_box.png", game_file_loader).unwrap();
        let expanded_arrow_texture = texture_loader.get("expanded_arrow.png", game_file_loader).unwrap();
        let collapsed_arrow_texture = texture_loader.get("collapsed_arrow.png", game_file_loader).unwrap();

        Self {
            memory_allocator,
            font_loader,
            queue,
            render_pass,
            rectangle_renderer,
            sprite_renderer,
            text_renderer: font_renderer,
            checked_box_texture,
            unchecked_box_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,
            dimensions,
        }
    }

    #[profile("re-create interface pipeline")]
    pub fn recreate_pipeline(&mut self, viewport: Viewport, dimensions: [u32; 2]) {
        let device = self.memory_allocator.device().clone();
        let subpass = self.render_pass.clone().first_subpass();

        self.rectangle_renderer
            .recreate_pipeline(device.clone(), subpass.clone(), viewport.clone());
        self.sprite_renderer
            .recreate_pipeline(device.clone(), subpass.clone(), viewport.clone());
        self.text_renderer.recreate_pipeline(device, subpass, viewport);
        self.dimensions = dimensions;
    }

    #[profile("create interface render target")]
    pub fn create_render_target(&self) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(
            self.memory_allocator.clone(),
            self.queue.clone(),
            self.render_pass.clone(),
            self.dimensions,
            SampleCount::Sample4,
            ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST | ImageUsage::COLOR_ATTACHMENT | ImageUsage::INPUT_ATTACHMENT,
            ClearColorValue::Float([0.0, 0.0, 0.0, 0.0]),
        )
    }

    fn get_window_size(&self) -> ScreenSize {
        ScreenSize {
            width: self.dimensions[0] as f32,
            height: self.dimensions[1] as f32,
        }
    }
}

impl korangar_interface::application::InterfaceRenderer<InterfaceSettings> for InterfaceRenderer {
    type Target = <Self as Renderer>::Target;

    fn get_text_dimensions(
        &self,
        text: &str,
        font_size: <InterfaceSettings as Application>::FontSize,
        available_width: f32,
    ) -> <InterfaceSettings as Application>::Size {
        self.font_loader.borrow().get_text_dimensions(text, font_size, available_width)
    }

    fn render_rectangle(
        &self,
        render_target: &mut Self::Target,
        position: <InterfaceSettings as Application>::Position,
        size: <InterfaceSettings as Application>::Size,
        clip: <InterfaceSettings as Application>::Clip,
        corner_radius: <InterfaceSettings as Application>::CornerRadius,
        color: <InterfaceSettings as Application>::Color,
    ) {
        self.rectangle_renderer.render(
            render_target,
            self.get_window_size(),
            position,
            size,
            clip,
            corner_radius,
            color,
        );
    }

    fn render_text(
        &self,
        render_target: &mut Self::Target,
        text: &str,
        position: <InterfaceSettings as Application>::Position,
        clip: <InterfaceSettings as Application>::Clip,
        color: <InterfaceSettings as Application>::Color,
        font_size: <InterfaceSettings as Application>::FontSize,
    ) -> f32 {
        self.text_renderer
            .render(render_target, text, self.get_window_size(), position, clip, color, font_size)
    }

    fn render_checkbox(
        &self,
        render_target: &mut Self::Target,
        position: <InterfaceSettings as Application>::Position,
        size: <InterfaceSettings as Application>::Size,
        clip: <InterfaceSettings as Application>::Clip,
        color: <InterfaceSettings as Application>::Color,
        checked: bool,
    ) {
        let texture = match checked {
            true => self.checked_box_texture.clone(),
            false => self.unchecked_box_texture.clone(),
        };

        self.render_sprite(render_target, texture, position, size, clip, color, true);
    }

    fn render_expand_arrow(
        &self,
        render_target: &mut Self::Target,
        position: <InterfaceSettings as Application>::Position,
        size: <InterfaceSettings as Application>::Size,
        clip: <InterfaceSettings as Application>::Clip,
        color: <InterfaceSettings as Application>::Color,
        expanded: bool,
    ) {
        let texture = match expanded {
            true => self.expanded_arrow_texture.clone(),
            false => self.collapsed_arrow_texture.clone(),
        };

        self.render_sprite(render_target, texture, position, size, clip, color, true);
    }
}

pub struct InterfaceFormat {}

impl IntoFormat for InterfaceFormat {
    fn into_format() -> Format {
        Format::R8G8B8A8_UNORM
    }
}

impl Renderer for InterfaceRenderer {
    type Target = SingleRenderTarget<InterfaceFormat, InterfaceSubrenderer, ClearColorValue>;
}

impl SpriteRendererTrait for InterfaceRenderer {
    fn render_sprite(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        texture: Arc<ImageView>,
        position: ScreenPosition,
        size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) where
        Self: Renderer,
    {
        self.sprite_renderer.render(
            render_target,
            texture,
            self.get_window_size(),
            position,
            size,
            screen_clip,
            color,
            smooth,
        );
    }
}
