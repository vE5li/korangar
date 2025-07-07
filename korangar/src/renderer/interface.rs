use std::cell::{Ref, RefCell};
use std::sync::Arc;

use cgmath::EuclideanSpace;
use korangar_interface::application::Application;

use crate::graphics::{Color, InterfaceRectangleInstruction, Texture};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{FontLoader, GlyphInstruction, ImageType, TextureLoader};
use crate::renderer::SpriteRenderer;

/// Renders the interface provided by 'korangar_interface'.
pub struct InterfaceRenderer {
    instructions: RefCell<Vec<InterfaceRectangleInstruction>>,
    glyphs: RefCell<Vec<GlyphInstruction>>,
    font_loader: Arc<FontLoader>,
    filled_box_texture: Arc<Texture>,
    unfilled_box_texture: Arc<Texture>,
    expanded_arrow_texture: Arc<Texture>,
    collapsed_arrow_texture: Arc<Texture>,
    window_size: ScreenSize,
    interface_size: ScreenSize,
    high_quality_interface: bool,
}

impl InterfaceRenderer {
    pub fn new(
        window_size: ScreenSize,
        font_loader: Arc<FontLoader>,
        texture_loader: &TextureLoader,
        high_quality_interface: bool,
    ) -> Self {
        let instructions = RefCell::new(Vec::default());
        let glyphs = RefCell::new(Vec::default());

        let filled_box_texture = texture_loader.get_or_load("filled_box.png", ImageType::Sdf).unwrap();
        let unfilled_box_texture = texture_loader.get_or_load("unfilled_box.png", ImageType::Sdf).unwrap();
        let expanded_arrow_texture = texture_loader.get_or_load("expanded_arrow.png", ImageType::Sdf).unwrap();
        let collapsed_arrow_texture = texture_loader.get_or_load("collapsed_arrow.png", ImageType::Sdf).unwrap();

        let interface_size = if high_quality_interface { window_size * 2.0 } else { window_size };

        Self {
            instructions,
            glyphs,
            font_loader,
            filled_box_texture,
            unfilled_box_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,
            window_size,
            interface_size,
            high_quality_interface,
        }
    }

    pub fn clear(&self) {
        self.instructions.borrow_mut().clear();
    }

    pub fn get_instructions(&self) -> Ref<'_, Vec<InterfaceRectangleInstruction>> {
        self.instructions.borrow()
    }

    pub fn update_high_quality_interface(&mut self, high_quality_interface: bool) {
        self.high_quality_interface = high_quality_interface;
        self.interface_size = if self.high_quality_interface {
            self.window_size * 2.0
        } else {
            self.window_size
        };
    }

    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.window_size = window_size;
        self.interface_size = if self.high_quality_interface {
            self.window_size * 2.0
        } else {
            self.window_size
        };
    }
}

impl korangar_interface::application::InterfaceRenderer<InterfaceSettings> for InterfaceRenderer {
    fn get_text_dimensions(
        &self,
        text: &str,
        mut font_size: <InterfaceSettings as Application>::FontSize,
        mut available_width: f32,
    ) -> <InterfaceSettings as Application>::Size {
        if self.high_quality_interface {
            // We need to adjust the font size, or else we would create glyphs for a font
            // size, that we don't use.
            font_size = font_size * 2.0;
            available_width *= 2.0;
        }

        let mut size = self.font_loader.get_text_dimensions(text, font_size, 1.0, available_width);

        if self.high_quality_interface {
            size = size / 2.0;
        }

        size
    }

    fn render_rectangle(
        &self,
        position: <InterfaceSettings as Application>::Position,
        size: <InterfaceSettings as Application>::Size,
        mut screen_clip: <InterfaceSettings as Application>::Clip,
        mut corner_radius: <InterfaceSettings as Application>::CornerRadius,
        color: <InterfaceSettings as Application>::Color,
    ) {
        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
            corner_radius = corner_radius * 2.0;
        }

        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;
        let corner_radius = corner_radius * 0.5;

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_radius,
        });
    }

    fn render_text(
        &self,
        text: &str,
        mut text_position: <InterfaceSettings as Application>::Position,
        mut screen_clip: <InterfaceSettings as Application>::Clip,
        color: <InterfaceSettings as Application>::Color,
        mut font_size: <InterfaceSettings as Application>::FontSize,
    ) -> f32 {
        if self.high_quality_interface {
            text_position = text_position * 2.0;
            screen_clip = screen_clip * 2.0;
            font_size = font_size * 2.0;
        }

        let mut glyphs = self.glyphs.borrow_mut();

        let mut size = self.font_loader.layout_text(
            text,
            color,
            font_size,
            1.0,
            screen_clip.right - text_position.left,
            Some(&mut glyphs),
        );

        let mut instructions = self.instructions.borrow_mut();

        glyphs.drain(..).for_each(
            |GlyphInstruction {
                 position,
                 texture_coordinate,
                 color,
             }| {
                let screen_position = ScreenPosition {
                    left: text_position.left + position.min.x,
                    top: text_position.top + position.min.y,
                } / self.interface_size;

                let screen_size = ScreenSize {
                    width: position.width(),
                    height: position.height(),
                } / self.interface_size;

                let texture_position = texture_coordinate.min.to_vec();
                let texture_size = texture_coordinate.max - texture_coordinate.min;

                instructions.push(InterfaceRectangleInstruction::Text {
                    screen_position,
                    screen_size,
                    screen_clip,
                    texture_position,
                    texture_size,
                    color,
                });
            },
        );

        if self.high_quality_interface {
            size.y /= 2.0;
        }

        size.y
    }

    fn render_checkbox(
        &self,
        position: <InterfaceSettings as Application>::Position,
        size: <InterfaceSettings as Application>::Size,
        clip: <InterfaceSettings as Application>::Clip,
        color: <InterfaceSettings as Application>::Color,
        checked: bool,
    ) {
        let texture = match checked {
            true => self.filled_box_texture.clone(),
            false => self.unfilled_box_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    fn render_expand_arrow(
        &self,
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

        self.render_sdf(texture, position, size, clip, color);
    }
}

impl SpriteRenderer for InterfaceRenderer {
    fn render_sprite(
        &self,
        texture: Arc<Texture>,
        position: ScreenPosition,
        size: ScreenSize,
        mut screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) {
        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
        }

        // Normalize screen_position and screen_size in range 0.0 and 1.0.
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;
        let corner_radius = CornerRadius::default();

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Sprite {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_radius,
            texture,
            smooth,
        });
    }

    fn render_sdf(&self, texture: Arc<Texture>, position: ScreenPosition, size: ScreenSize, mut screen_clip: ScreenClip, color: Color) {
        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
        }

        // Normalize screen_position and screen_size in range 0.0 and 1.0.
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;
        let corner_radius = CornerRadius::default();

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Sdf {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_radius,
            texture,
        });
    }
}
