mod rectangle;
mod sprite;
mod text;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use korangar_interface::application::Application;
use wgpu::{Device, RenderPass, TextureFormat, TextureUsages};

use self::rectangle::RectangleRenderer;
use self::sprite::SpriteRenderer;
use self::text::TextRenderer;
use super::IntoFormat;
use crate::graphics::{Color, Renderer, SingleRenderTarget, SpriteRenderer as SpriteRendererTrait, Texture};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{FontLoader, TextureLoader};

#[derive(PartialEq, Eq)]
pub enum InterfaceSubRenderer {
    Rectangle,
    Sprite,
    Text,
}

pub struct InterfaceRenderer {
    device: Arc<Device>,
    font_loader: Rc<RefCell<FontLoader>>,
    rectangle_renderer: RectangleRenderer,
    sprite_renderer: SpriteRenderer,
    text_renderer: TextRenderer,
    checked_box_texture: Arc<Texture>,
    unchecked_box_texture: Arc<Texture>,
    expanded_arrow_texture: Arc<Texture>,
    collapsed_arrow_texture: Arc<Texture>,
    dimensions: [u32; 2],
}

impl InterfaceRenderer {
    pub fn new(
        device: Arc<Device>,
        texture_loader: &mut TextureLoader,
        font_loader: Rc<RefCell<FontLoader>>,
        dimensions: [u32; 2],
    ) -> Self {
        let output_texture_format = <Self as Renderer>::Target::output_texture_format();

        let rectangle_renderer = RectangleRenderer::new(device.clone(), output_texture_format);
        let sprite_renderer = SpriteRenderer::new(device.clone(), output_texture_format);
        let text_renderer = TextRenderer::new(device.clone(), output_texture_format, font_loader.clone());

        let checked_box_texture = texture_loader.get("checked_box.png").unwrap();
        let unchecked_box_texture = texture_loader.get("unchecked_box.png").unwrap();
        let expanded_arrow_texture = texture_loader.get("expanded_arrow.png").unwrap();
        let collapsed_arrow_texture = texture_loader.get("collapsed_arrow.png").unwrap();

        Self {
            device,
            font_loader,
            rectangle_renderer,
            sprite_renderer,
            text_renderer,
            checked_box_texture,
            unchecked_box_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,
            dimensions,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("reconfigure interface pipeline"))]
    pub fn reconfigure_pipeline(&mut self, dimensions: [u32; 2]) {
        self.dimensions = dimensions;
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("create interface render target"))]
    pub fn create_render_target(&self) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(
            &self.device,
            "interface",
            self.dimensions,
            4,
            TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
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
        render_pass: &mut RenderPass,
        position: <InterfaceSettings as Application>::Position,
        size: <InterfaceSettings as Application>::Size,
        clip: <InterfaceSettings as Application>::Clip,
        corner_radius: <InterfaceSettings as Application>::CornerRadius,
        color: <InterfaceSettings as Application>::Color,
    ) {
        self.rectangle_renderer.render(
            render_target,
            render_pass,
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
        render_pass: &mut RenderPass,
        text: &str,
        position: <InterfaceSettings as Application>::Position,
        clip: <InterfaceSettings as Application>::Clip,
        color: <InterfaceSettings as Application>::Color,
        font_size: <InterfaceSettings as Application>::FontSize,
    ) -> f32 {
        self.text_renderer.render(
            render_target,
            render_pass,
            text,
            self.get_window_size(),
            position,
            clip,
            color,
            font_size,
        )
    }

    fn render_checkbox(
        &self,
        render_target: &mut Self::Target,
        render_pass: &mut RenderPass,
        position: <InterfaceSettings as Application>::Position,
        size: <InterfaceSettings as Application>::Size,
        clip: <InterfaceSettings as Application>::Clip,
        color: <InterfaceSettings as Application>::Color,
        checked: bool,
    ) {
        let texture = match checked {
            true => &self.checked_box_texture,
            false => &self.unchecked_box_texture,
        };

        self.render_sprite(render_target, render_pass, texture, position, size, clip, color, true);
    }

    fn render_expand_arrow(
        &self,
        render_target: &mut Self::Target,
        render_pass: &mut RenderPass,
        position: <InterfaceSettings as Application>::Position,
        size: <InterfaceSettings as Application>::Size,
        clip: <InterfaceSettings as Application>::Clip,
        color: <InterfaceSettings as Application>::Color,
        expanded: bool,
    ) {
        let texture = match expanded {
            true => &self.expanded_arrow_texture,
            false => &self.collapsed_arrow_texture,
        };

        self.render_sprite(render_target, render_pass, texture, position, size, clip, color, true);
    }
}

pub struct InterfaceFormat {}

impl IntoFormat for InterfaceFormat {
    fn into_format() -> TextureFormat {
        TextureFormat::Rgba8UnormSrgb
    }
}

impl Renderer for InterfaceRenderer {
    type Target = SingleRenderTarget<InterfaceFormat, InterfaceSubRenderer, wgpu::Color>;
}

impl SpriteRendererTrait for InterfaceRenderer {
    fn render_sprite(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        texture: &Texture,
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
            render_pass,
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
