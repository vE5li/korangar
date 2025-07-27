use std::cell::{Ref, RefCell};
use std::sync::Arc;

use cgmath::EuclideanSpace;
use korangar_interface::application::RenderLayer;
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{ClipLayer, ClipLayerId, Icon, Layout};

use crate::graphics::{Color, InterfaceRectangleInstruction, Texture};
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{FontLoader, FontSize, GlyphInstruction, ImageType, Sprite, TextureLoader};
use crate::renderer::SpriteRenderer;
use crate::state::ClientState;
use crate::world::{Actions, SpriteAnimationState};

/// Renders the interface provided by 'korangar_interface'.
pub struct InterfaceRenderer {
    instructions: RefCell<Vec<InterfaceRectangleInstruction>>,
    glyphs: RefCell<Vec<GlyphInstruction>>,
    font_loader: Arc<FontLoader>,
    filled_box_texture: Arc<Texture>,
    unfilled_box_texture: Arc<Texture>,
    expanded_arrow_texture: Arc<Texture>,
    collapsed_arrow_texture: Arc<Texture>,
    eye_open_texture: Arc<Texture>,
    eye_closed_texture: Arc<Texture>,
    trash_can_texture: Arc<Texture>,
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
        let eye_open_texture = texture_loader.get_or_load("eye_open.png", ImageType::Sdf).unwrap();
        let eye_closed_texture = texture_loader.get_or_load("eye_closed.png", ImageType::Sdf).unwrap();
        let trash_can_texture = texture_loader.get_or_load("trash_can.png", ImageType::Sdf).unwrap();

        let interface_size = if high_quality_interface { window_size * 2.0 } else { window_size };

        Self {
            instructions,
            glyphs,
            font_loader,
            filled_box_texture,
            unfilled_box_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,
            eye_open_texture,
            eye_closed_texture,
            trash_can_texture,
            window_size,
            interface_size,
            high_quality_interface,
        }
    }

    pub fn clear(&self) {
        self.instructions.borrow_mut().clear();
    }

    pub fn get_instructions(&self) -> Ref<Vec<InterfaceRectangleInstruction>> {
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

    pub fn get_text_dimensions(&self, text: &str, mut font_size: FontSize, mut available_width: f32) -> ScreenSize {
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

    pub fn render_rectangle(
        &self,
        position: ScreenPosition,
        size: ScreenSize,
        mut screen_clip: ScreenClip,
        mut corner_radius: CornerRadius,
        color: Color,
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

    pub fn render_text(
        &self,
        text: &str,
        mut text_position: ScreenPosition,
        mut screen_clip: ScreenClip,
        color: Color,
        mut font_size: FontSize,
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

    pub fn render_checkbox(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, checked: bool) {
        let texture = match checked {
            true => self.filled_box_texture.clone(),
            false => self.unfilled_box_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    pub fn render_expand_arrow(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, expanded: bool) {
        let texture = match expanded {
            true => self.expanded_arrow_texture.clone(),
            false => self.collapsed_arrow_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    pub fn render_eye(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, open: bool) {
        let texture = match open {
            true => self.eye_open_texture.clone(),
            false => self.eye_closed_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    pub fn render_trash_can(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color) {
        self.render_sdf(self.trash_can_texture.clone(), position, size, clip, color);
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

/// An instruction to render a texture.
///
/// These are not used outside this module but are exposed through
/// [`CustomInstruction`]. Thus we explicitly allow a private interface.
#[allow(private_interfaces)]
struct TextureInstruction {
    texture: Arc<Texture>,
    clip_layer: ClipLayerId,
    area: Area,
    color: Color,
    smooth: bool,
}

/// An instruction to render a sprite.
///
/// These are not used outside this module but are exposed through
/// [`CustomInstruction`]. Thus we explicitly allow a private interface.
#[allow(private_interfaces)]
struct SpriteInstruction<'a> {
    actions: &'a Actions,
    sprite: &'a Sprite,
    animation_state: &'a SpriteAnimationState,
    clip_layer: ClipLayerId,
    area: Area,
    color: Color,
    smooth: bool,
}

pub enum CustomInstruction<'a> {
    Texture(TextureInstruction),
    Sprite(SpriteInstruction<'a>),
}

impl RenderLayer<ClientState> for InterfaceRenderer {
    type CustomIcon = ();
    type CustomInstruction<'a> = CustomInstruction<'a>;

    fn render_rectangle(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, corner_radius: CornerRadius, color: Color) {
        self.render_rectangle(position, size, clip, corner_radius, color);
    }

    fn get_text_dimensions(&self, text: &str, font_size: FontSize, available_width: f32) -> ScreenSize {
        self.get_text_dimensions(text, font_size, available_width)
    }

    fn render_text(&self, text: &str, position: ScreenPosition, clip: ScreenClip, color: Color, font_size: FontSize) {
        self.render_text(text, position, clip, color, font_size);
    }

    fn render_icon(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, icon: Icon<ClientState>, color: Color) {
        match icon {
            Icon::ExpandArrow { expanded } => self.render_expand_arrow(position, size, clip, color, expanded),
            Icon::Checkbox { checked } => self.render_checkbox(position, size, clip, color, checked),
            Icon::Eye { open } => self.render_eye(position, size, clip, color, open),
            Icon::TrashCan => self.render_trash_can(position, size, clip, color),
            Icon::Custom(_) => (),
        }
    }

    fn render_custom(&self, instruction: Self::CustomInstruction<'_>, clip_layers: &[ClipLayer<ClientState>]) {
        match instruction {
            CustomInstruction::Sprite(SpriteInstruction {
                actions,
                sprite,
                animation_state,
                clip_layer,
                area,
                color,
                smooth,
            }) => {
                let position = ScreenPosition {
                    left: area.left + area.width / 2.0,
                    top: area.top + area.height / 2.0,
                };
                let screen_clip = clip_layers[clip_layer.0].get();

                actions.render_sprite(self, sprite, animation_state, position, 0, screen_clip, color, 1.0);
            }
            CustomInstruction::Texture(TextureInstruction {
                texture,
                clip_layer,
                area,
                color,
                smooth,
            }) => {
                let position = ScreenPosition {
                    left: area.left,
                    top: area.top,
                };
                let size = ScreenSize {
                    width: area.width,
                    height: area.height,
                };
                let screen_clip = clip_layers[clip_layer.0].get();

                self.render_sprite(texture, position, size, screen_clip, color, smooth);
            }
        }
    }
}

pub trait LayoutExt<'a> {
    fn add_texture(&mut self, texture: Arc<Texture>, area: Area, color: Color, smooth: bool);

    fn add_sprite(
        &mut self,
        actions: &'a Actions,
        sprite: &'a Sprite,
        animation_state: &'a SpriteAnimationState,
        area: Area,
        color: Color,
        smooth: bool,
    );
}

impl<'a> LayoutExt<'a> for Layout<'a, ClientState> {
    fn add_texture(&mut self, texture: Arc<Texture>, area: Area, color: Color, smooth: bool) {
        let clip_layer = self.get_active_clip_layer();

        self.add_custom_instruction(CustomInstruction::Texture(TextureInstruction {
            texture,
            clip_layer,
            area,
            color,
            smooth,
        }));
    }

    fn add_sprite(
        &mut self,
        actions: &'a Actions,
        sprite: &'a Sprite,
        animation_state: &'a SpriteAnimationState,
        area: Area,
        color: Color,
        smooth: bool,
    ) {
        let clip_layer = self.get_active_clip_layer();

        self.add_custom_instruction(CustomInstruction::Sprite(SpriteInstruction {
            actions,
            sprite,
            animation_state,
            clip_layer,
            area,
            color,
            smooth,
        }));
    }
}
