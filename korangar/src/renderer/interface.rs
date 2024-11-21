use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use cgmath::Vector2;
use korangar_interface::application::Application;

use crate::graphics::{Color, InterfaceRectangleInstruction, Texture};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{FontLoader, TextureLoader};
use crate::renderer::SpriteRenderer;

/// Renders the interface provided by 'korangar_interface'.
pub struct InterfaceRenderer {
    instructions: RefCell<Vec<InterfaceRectangleInstruction>>,
    font_loader: Rc<RefCell<FontLoader>>,
    checked_box_texture: Arc<Texture>,
    unchecked_box_texture: Arc<Texture>,
    expanded_arrow_texture: Arc<Texture>,
    collapsed_arrow_texture: Arc<Texture>,
    window_size: ScreenSize,
}

impl InterfaceRenderer {
    pub fn new(window_size: ScreenSize, font_loader: Rc<RefCell<FontLoader>>, texture_loader: &TextureLoader) -> Self {
        let instructions = RefCell::new(Vec::default());

        let checked_box_texture = texture_loader.get("checked_box.png").unwrap();
        let unchecked_box_texture = texture_loader.get("unchecked_box.png").unwrap();
        let expanded_arrow_texture = texture_loader.get("expanded_arrow.png").unwrap();
        let collapsed_arrow_texture = texture_loader.get("collapsed_arrow.png").unwrap();

        Self {
            instructions,
            font_loader,
            checked_box_texture,
            unchecked_box_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,
            window_size,
        }
    }

    pub fn clear(&self) {
        self.instructions.borrow_mut().clear();
    }

    pub fn get_instructions(&self) -> Ref<Vec<InterfaceRectangleInstruction>> {
        self.instructions.borrow()
    }

    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.window_size = window_size;
    }
}

impl korangar_interface::application::InterfaceRenderer<InterfaceSettings> for InterfaceRenderer {
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
        position: <InterfaceSettings as Application>::Position,
        size: <InterfaceSettings as Application>::Size,
        screen_clip: <InterfaceSettings as Application>::Clip,
        corner_radius: <InterfaceSettings as Application>::CornerRadius,
        color: <InterfaceSettings as Application>::Color,
    ) {
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;
        // TODO: NHA It seems that corners are currently defined as "double" their
        //       actual size. We currently compensate for that.
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
        position: <InterfaceSettings as Application>::Position,
        clip: <InterfaceSettings as Application>::Clip,
        color: <InterfaceSettings as Application>::Color,
        font_size: <InterfaceSettings as Application>::FontSize,
    ) -> f32 {
        let mut font_loader = self.font_loader.borrow_mut();
        let (character_layout, height) = font_loader.get(text, color, font_size, clip.right - position.left);

        character_layout.iter().for_each(|(texture_coordinates, glyph_position, color)| {
            let screen_position = ScreenPosition {
                left: position.left + glyph_position.min.x as f32,
                top: position.top + glyph_position.min.y as f32,
            } / self.window_size;

            let screen_size = ScreenSize {
                width: glyph_position.width() as f32,
                height: glyph_position.height() as f32,
            } / self.window_size;

            let texture_position = texture_coordinates.min;
            // TODO: use absolute instead
            let texture_size = texture_coordinates.max - texture_coordinates.min;

            self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Text {
                screen_position,
                screen_size,
                screen_clip: clip,
                texture_position: Vector2::new(texture_position.x, texture_position.y),
                texture_size: Vector2::new(texture_size.x, texture_size.y),
                color: *color,
            });
        });

        height
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
            true => self.checked_box_texture.clone(),
            false => self.unchecked_box_texture.clone(),
        };

        self.render_sprite(texture, position, size, clip, color, true);
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

        self.render_sprite(texture, position, size, clip, color, true);
    }
}

impl SpriteRenderer for InterfaceRenderer {
    fn render_sprite(
        &self,
        texture: Arc<Texture>,
        position: ScreenPosition,
        size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) {
        // Normalize screen_position and screen_size in range 0.0 and 1.0.
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Sprite {
            screen_position,
            screen_size,
            screen_clip,
            color,
            texture,
            smooth,
        });
    }
}
