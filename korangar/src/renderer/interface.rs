use std::cell::{Ref, RefCell};
use std::sync::Arc;

use cgmath::EuclideanSpace;
#[cfg(feature = "debug")]
use korangar_interface::InterfaceFrame;
#[cfg(feature = "debug")]
use korangar_interface::application::Clip;
use korangar_interface::application::RenderLayer;
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{ClipLayer, ClipLayerId, Icon, WindowLayout};

use crate::graphics::{Color, CornerDiameter, InterfaceRectangleInstruction, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::loaders::{FontLoader, FontSize, GlyphInstruction, ImageType, OverflowBehavior, Sprite, TextureLoader};
use crate::renderer::SpriteRenderer;
use crate::state::ClientState;
use crate::world::{Actions, SpriteAnimationState};

/// Renders the interface provided by [`korangar_interface`].
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
    #[cfg(feature = "debug")]
    show_rectangle_instructions: bool,
    #[cfg(feature = "debug")]
    show_glyph_instructions: bool,
    #[cfg(feature = "debug")]
    show_sprite_instructions: bool,
    #[cfg(feature = "debug")]
    show_sdf_instructions: bool,
}

impl InterfaceRenderer {
    /// Create a new interface renderer.
    ///
    /// This include loading the textures icons for rendering components.
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

        #[cfg(feature = "debug")]
        let show_rectangle_instructions = false;
        #[cfg(feature = "debug")]
        let show_glyph_instructions = false;
        #[cfg(feature = "debug")]
        let show_sprite_instructions = false;
        #[cfg(feature = "debug")]
        let show_sdf_instructions = false;

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
            #[cfg(feature = "debug")]
            show_rectangle_instructions,
            #[cfg(feature = "debug")]
            show_glyph_instructions,
            #[cfg(feature = "debug")]
            show_sprite_instructions,
            #[cfg(feature = "debug")]
            show_sdf_instructions,
        }
    }

    #[cfg(feature = "debug")]
    pub fn update_render_options(&mut self, render_options: &crate::RenderOptions) {
        self.show_rectangle_instructions = render_options.show_rectangle_instructions;
        self.show_glyph_instructions = render_options.show_glyph_instructions;
        self.show_sprite_instructions = render_options.show_sprite_instructions;
        self.show_sdf_instructions = render_options.show_sdf_instructions;
    }

    /// Don't show rectangle instructions for the duration of the closure. This
    /// is only used to render other debug areas.
    #[cfg(feature = "debug")]
    pub fn with_rectangles_direct(&mut self, f: impl Fn(&Self)) {
        let previous_state = self.show_rectangle_instructions;
        self.show_rectangle_instructions = false;

        f(self);

        self.show_rectangle_instructions = previous_state;
    }

    /// Clear render instructions.
    pub fn clear(&self) {
        self.instructions.borrow_mut().clear();
    }

    /// Get render instructions.
    pub fn get_instructions(&self) -> Ref<'_, Vec<InterfaceRectangleInstruction>> {
        self.instructions.borrow()
    }

    /// Inform the renderer of a change in the high quality interface setting.
    pub fn update_high_quality_interface(&mut self, high_quality_interface: bool) {
        self.high_quality_interface = high_quality_interface;
        self.interface_size = if self.high_quality_interface {
            self.window_size * 2.0
        } else {
            self.window_size
        };
    }

    /// Inform the renderer of new window size.
    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.window_size = window_size;
        self.interface_size = if self.high_quality_interface {
            self.window_size * 2.0
        } else {
            self.window_size
        };
    }

    /// Get the bounds of a given text, respecting the loaded font.
    pub fn get_text_dimensions(
        &self,
        text: &str,
        color: Color,
        highlight_color: Color,
        mut font_size: FontSize,
        mut available_width: f32,
        overflow_behavior: OverflowBehavior,
    ) -> (ScreenSize, FontSize) {
        if self.high_quality_interface {
            // We need to adjust the font size, or else we would create glyphs for a font
            // size, that we don't use.
            font_size = font_size * 2.0;
            available_width *= 2.0;
        }

        let (mut size, mut font_size) =
            self.font_loader
                .get_text_dimensions(text, color, highlight_color, font_size, 1.0, available_width, overflow_behavior);

        if self.high_quality_interface {
            size = size / 2.0;
            font_size = FontSize(font_size.0 / 2.0);
        }

        (size, font_size)
    }

    /// Add instruction for rendering a rectangle.
    pub fn render_rectangle(
        &self,
        position: ScreenPosition,
        size: ScreenSize,
        mut screen_clip: ScreenClip,
        mut corner_diameter: CornerDiameter,
        color: Color,
    ) {
        // If the rectangle is not even within the bounds of the clip, discard it early
        // saving GPU resources.
        if position.left > screen_clip.right
            || position.top > screen_clip.bottom
            || position.left + size.width < screen_clip.left
            || position.top + size.height < screen_clip.top
        {
            #[cfg(feature = "debug")]
            if self.show_rectangle_instructions {
                let screen_position = position / self.window_size;
                let screen_size = size / self.window_size;

                self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                    screen_position,
                    screen_size,
                    screen_clip: ScreenClip::unbound(),
                    color: Color::rgba_u8(255, 0, 0, 90),
                    corner_diameter: CornerDiameter::default(),
                });
            }

            return;
        }

        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
            corner_diameter = corner_diameter * 2.0;
        }

        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        #[cfg(feature = "debug")]
        if self.show_rectangle_instructions {
            // We make it easier to differentiate the rectangles by adjusting the color
            // based on the size.
            let brightness = (1500.0 - size.height).max(0.0) / 1500.0;

            self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                screen_position,
                screen_size,
                screen_clip,
                color: Color::rgba(1.0 - brightness, 1.0 - brightness, 1.0 - brightness, 0.6),
                corner_diameter: CornerDiameter::default(),
            });

            return;
        }

        let corner_diameter = corner_diameter * 0.5;

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_diameter,
        });
    }

    /// Add instructions for rendering glyphs.
    pub fn render_text(
        &self,
        text: &str,
        mut text_position: ScreenPosition,
        mut available_width: f32,
        mut screen_clip: ScreenClip,
        color: Color,
        highlight_color: Color,
        mut font_size: FontSize,
    ) -> f32 {
        // TODO: Can't we scale after laying out the text? Would cut down on
        // multiplications.
        if self.high_quality_interface {
            text_position = text_position * 2.0;
            available_width *= 2.0;
            screen_clip = screen_clip * 2.0;
            font_size = font_size * 2.0;
        }

        let mut glyphs = self.glyphs.borrow_mut();

        let mut size = self.font_loader.layout_text(
            text,
            color,
            highlight_color,
            font_size,
            1.0,
            Some(available_width),
            Some(&mut glyphs),
        );

        let mut instructions = self.instructions.borrow_mut();

        glyphs.drain(..).for_each(
            |GlyphInstruction {
                 position,
                 texture_coordinate,
                 color,
             }| {
                // If the character is not even within the bounds of the clip, discard it early
                // saving GPU resources.
                //
                // TODO: For some reason the min.y is actually max.y and vice versa. Not sure
                // how this rendering code works but that's why the check is
                // using max.y and min.y inverted.
                if text_position.left + position.min.x > screen_clip.right
                    || text_position.top + position.max.y > screen_clip.bottom
                    || text_position.left + position.max.x < screen_clip.left
                    || text_position.top + position.min.y < screen_clip.top
                {
                    #[cfg(feature = "debug")]
                    if self.show_glyph_instructions {
                        let screen_position = ScreenPosition {
                            left: text_position.left + position.min.x,
                            top: text_position.top + position.min.y,
                        } / self.interface_size;

                        let screen_size = ScreenSize {
                            width: position.width(),
                            height: position.height(),
                        } / self.interface_size;

                        instructions.push(InterfaceRectangleInstruction::Solid {
                            screen_position,
                            screen_size,
                            screen_clip: ScreenClip::unbound(),
                            color: Color::rgba_u8(255, 0, 0, 150),
                            corner_diameter: CornerDiameter::default(),
                        });
                    }

                    return;
                }

                let screen_position = ScreenPosition {
                    left: text_position.left + position.min.x,
                    top: text_position.top + position.min.y,
                } / self.interface_size;

                let screen_size = ScreenSize {
                    width: position.width(),
                    height: position.height(),
                } / self.interface_size;

                #[cfg(feature = "debug")]
                if self.show_glyph_instructions {
                    instructions.push(InterfaceRectangleInstruction::Solid {
                        screen_position,
                        screen_size,
                        screen_clip: ScreenClip::unbound(),
                        color: Color::rgba_u8(0, 255, 255, 150),
                        corner_diameter: CornerDiameter::default(),
                    });

                    return;
                }

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

    /// Render a checkbox icon using an SDF.
    pub fn render_checkbox(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, checked: bool) {
        let texture = match checked {
            true => self.filled_box_texture.clone(),
            false => self.unfilled_box_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    /// Render an expand arrow icon using an SDF.
    pub fn render_expand_arrow(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, expanded: bool) {
        let texture = match expanded {
            true => self.expanded_arrow_texture.clone(),
            false => self.collapsed_arrow_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    /// Render an eye icon using an SDF.
    pub fn render_eye(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, color: Color, open: bool) {
        let texture = match open {
            true => self.eye_open_texture.clone(),
            false => self.eye_closed_texture.clone(),
        };

        self.render_sdf(texture, position, size, clip, color);
    }

    /// Render a trash can icon using an SDF.
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
        // If the sprite is not even within the bounds of the clip, discard it early
        // saving GPU resources.
        if position.left > screen_clip.right
            || position.top > screen_clip.bottom
            || position.left + size.width < screen_clip.left
            || position.top + size.height < screen_clip.top
        {
            #[cfg(feature = "debug")]
            if self.show_sprite_instructions {
                let screen_position = position / self.window_size;
                let screen_size = size / self.window_size;

                self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                    screen_position,
                    screen_size,
                    screen_clip: ScreenClip::unbound(),
                    color: Color::rgba_u8(255, 0, 0, 150),
                    corner_diameter: CornerDiameter::default(),
                });
            }

            return;
        }

        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
        }

        // Normalize screen_position and screen_size in range 0.0 and 1.0.
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        #[cfg(feature = "debug")]
        if self.show_sprite_instructions {
            self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                screen_position,
                screen_size,
                screen_clip: ScreenClip::unbound(),
                color: Color::rgba_u8(255, 0, 255, 100),
                corner_diameter: CornerDiameter::default(),
            });

            return;
        }

        let corner_diameter = CornerDiameter::default();

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Sprite {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_diameter,
            texture,
            smooth,
        });
    }

    fn render_sdf(&self, texture: Arc<Texture>, position: ScreenPosition, size: ScreenSize, mut screen_clip: ScreenClip, color: Color) {
        // If the SDF is not even within the bounds of the clip, discard it early
        // saving GPU resources.
        if position.left > screen_clip.right
            || position.top > screen_clip.bottom
            || position.left + size.width < screen_clip.left
            || position.top + size.height < screen_clip.top
        {
            #[cfg(feature = "debug")]
            if self.show_sdf_instructions {
                let screen_position = position / self.window_size;
                let screen_size = size / self.window_size;

                self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                    screen_position,
                    screen_size,
                    screen_clip: ScreenClip::unbound(),
                    color: Color::rgba_u8(255, 0, 0, 100),
                    corner_diameter: CornerDiameter::default(),
                });
            }

            return;
        }

        if self.high_quality_interface {
            screen_clip = screen_clip * 2.0;
        }

        // Normalize screen_position and screen_size in range 0.0 and 1.0.
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        #[cfg(feature = "debug")]
        if self.show_sdf_instructions {
            self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Solid {
                screen_position,
                screen_size,
                screen_clip: ScreenClip::unbound(),
                color: Color::rgba_u8(255, 255, 0, 150),
                corner_diameter: CornerDiameter::default(),
            });

            return;
        }

        let corner_diameter = CornerDiameter::default();

        self.instructions.borrow_mut().push(InterfaceRectangleInstruction::Sdf {
            screen_position,
            screen_size,
            screen_clip,
            color,
            corner_diameter,
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
    scaling: f32,
}

/// A custom layout instruction.
///
/// Only pub to make the compiler happy, its not used outside of this module.
#[allow(private_interfaces)]
pub enum CustomInstruction<'a> {
    /// An instruction to render a texture.
    Texture(TextureInstruction),
    /// An instruction to render a sprite.
    Sprite(SpriteInstruction<'a>),
}

impl RenderLayer<ClientState> for InterfaceRenderer {
    type CustomIcon = ();
    type CustomInstruction<'a> = CustomInstruction<'a>;

    fn render_rectangle(
        &self,
        position: ScreenPosition,
        size: ScreenSize,
        clip: ScreenClip,
        corner_diameter: CornerDiameter,
        color: Color,
    ) {
        self.render_rectangle(position, size, clip, corner_diameter, color);
    }

    fn render_text(
        &self,
        text: &str,
        position: ScreenPosition,
        available_width: f32,
        clip: ScreenClip,
        color: Color,
        highlight_color: Color,
        font_size: FontSize,
    ) {
        self.render_text(text, position, available_width, clip, color, highlight_color, font_size);
    }

    fn render_icon(&self, position: ScreenPosition, size: ScreenSize, clip: ScreenClip, icon: Icon<ClientState>, color: Color) {
        match icon {
            Icon::ExpandArrow { expanded } => self.render_expand_arrow(position, size, clip, color, expanded),
            Icon::Checkbox { checked } => self.render_checkbox(position, size, clip, color, checked),
            Icon::Eye { open } => self.render_eye(position, size, clip, color, open),
            Icon::TrashCan => self.render_trash_can(position, size, clip, color),
            Icon::Custom { .. } => {}
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
                scaling,
            }) => {
                let position = ScreenPosition {
                    left: area.left + area.width / 2.0,
                    top: area.top + area.height / 2.0,
                };
                let screen_clip = clip_layers[clip_layer.as_index()].get();

                actions.render_sprite(self, sprite, animation_state, position, 0, screen_clip, color, scaling);
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
                let screen_clip = clip_layers[clip_layer.as_index()].get();

                self.render_sprite(texture, position, size, screen_clip, color, smooth);
            }
        }
    }
}

/// Extension trait to make adding custom instructions to the [`Layout`]
/// seamless by mirroring its API.
pub trait LayoutExt<'a> {
    /// Add an instruction to render a texture.
    fn add_texture(&mut self, area: Area, texture: Arc<Texture>, color: Color, smooth: bool);

    /// Add an instruction to render a sprite.
    fn add_sprite(&mut self, area: Area, actions: &'a Actions, sprite: &'a Sprite, animation_state: &'a SpriteAnimationState, color: Color);
}

impl<'a> LayoutExt<'a> for WindowLayout<'a, ClientState> {
    fn add_texture(&mut self, area: Area, texture: Arc<Texture>, color: Color, smooth: bool) {
        let clip_layer = self.get_active_clip_layer();
        let area = self.scale_area(area);

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
        area: Area,
        actions: &'a Actions,
        sprite: &'a Sprite,
        animation_state: &'a SpriteAnimationState,
        color: Color,
    ) {
        let clip_layer = self.get_active_clip_layer();
        let area = self.scale_area(area);
        let scaling = self.get_interface_scaling();

        self.add_custom_instruction(CustomInstruction::Sprite(SpriteInstruction {
            actions,
            sprite,
            animation_state,
            clip_layer,
            area,
            color,
            scaling,
        }));
    }
}

/// Extesion trait for the [`InterfaceFrame`].
#[cfg(feature = "debug")]
pub trait InterfaceFrameExt {
    fn render_areas(&self, renderer: &InterfaceRenderer, render_options: &crate::RenderOptions);
}

#[cfg(feature = "debug")]
impl InterfaceFrameExt for InterfaceFrame<'_, ClientState> {
    fn render_areas(&self, renderer: &InterfaceRenderer, render_options: &crate::RenderOptions) {
        if render_options.show_click_areas {
            self.render_click_areas(renderer, Color::rgba_u8(255, 166, 0, 150));
        }

        if render_options.show_drop_areas {
            self.render_drop_areas(renderer, Color::rgba_u8(140, 255, 0, 150));
        }

        if render_options.show_scroll_areas {
            self.render_scroll_areas(renderer, Color::rgba_u8(166, 0, 255, 100));
        }
    }
}
