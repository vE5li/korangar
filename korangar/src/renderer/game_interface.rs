use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::sync::Arc;

#[cfg(feature = "debug")]
use cgmath::Point3;
use cgmath::{EuclideanSpace, Vector2};
use korangar_interface::application::FontSizeTraitExt;

use crate::graphics::{Color, RectangleInstruction, Texture};
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{FontLoader, FontSize, GlyphInstruction, Scaling, TextLayout};
#[cfg(feature = "debug")]
use crate::loaders::{ImageType, TextureLoader};
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
use crate::renderer::SpriteRenderer;
#[cfg(feature = "debug")]
use crate::world::Camera;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[allow(unused)]
pub enum AlignHorizontal {
    Left,
    Mid,
}

/// Renders the in-game interface (like the FPS counter, the mouse pointer or
/// the health bars).
pub struct GameInterfaceRenderer {
    instructions: RefCell<Vec<RectangleInstruction>>,
    font_loader: Rc<RefCell<FontLoader>>,
    window_size: ScreenSize,
    scaling: Scaling,
    #[cfg(feature = "debug")]
    object_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    light_source_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    sound_source_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    effect_source_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    entity_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    shadow_marker_texture: Arc<Texture>,
}

impl SpriteRenderer for GameInterfaceRenderer {
    fn render_sprite(
        &self,
        texture: Arc<Texture>,
        position: ScreenPosition,
        size: ScreenSize,
        _screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) {
        self.render_indexed(texture, position, size, color, 1, 0, smooth);
    }

    fn render_sdf(
        &self,
        texture: Arc<Texture>,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        _screen_clip: ScreenClip,
        color: Color,
    ) {
        let screen_position = ScreenPosition {
            left: screen_position.left / self.window_size.width,
            top: screen_position.top / self.window_size.height,
        };

        let screen_size = ScreenSize {
            width: screen_size.width / self.window_size.width,
            height: screen_size.height / self.window_size.height,
        };

        let texture_position = Vector2::new(0.0, 0.0);
        let texture_size = Vector2::new(1.0, 1.0);

        self.instructions.borrow_mut().push(RectangleInstruction::Sdf {
            screen_position,
            screen_size,
            color,
            texture_position,
            texture_size,
            texture,
        });
    }
}

impl GameInterfaceRenderer {
    pub fn new(
        window_size: ScreenSize,
        scaling: Scaling,
        font_loader: Rc<RefCell<FontLoader>>,
        #[cfg(feature = "debug")] texture_loader: &TextureLoader,
    ) -> Self {
        let instructions = RefCell::new(Vec::new());

        #[cfg(feature = "debug")]
        let object_marker_texture = texture_loader.get("marker_object.png", ImageType::Sdf).unwrap();
        #[cfg(feature = "debug")]
        let light_source_marker_texture = texture_loader.get("marker_light.png", ImageType::Sdf).unwrap();
        #[cfg(feature = "debug")]
        let sound_source_marker_texture = texture_loader.get("marker_sound.png", ImageType::Sdf).unwrap();
        #[cfg(feature = "debug")]
        let effect_source_marker_texture = texture_loader.get("marker_effect.png", ImageType::Sdf).unwrap();
        #[cfg(feature = "debug")]
        let entity_marker_texture = texture_loader.get("marker_entity.png", ImageType::Sdf).unwrap();
        #[cfg(feature = "debug")]
        let shadow_marker_texture = texture_loader.get("marker_shadow.png", ImageType::Sdf).unwrap();

        Self {
            instructions,
            font_loader,
            window_size,
            scaling,
            #[cfg(feature = "debug")]
            object_marker_texture,
            #[cfg(feature = "debug")]
            light_source_marker_texture,
            #[cfg(feature = "debug")]
            sound_source_marker_texture,
            #[cfg(feature = "debug")]
            effect_source_marker_texture,
            #[cfg(feature = "debug")]
            entity_marker_texture,
            #[cfg(feature = "debug")]
            shadow_marker_texture,
        }
    }

    pub fn from_renderer(other: &Self) -> Self {
        Self {
            instructions: RefCell::new(Vec::default()),
            font_loader: Rc::clone(&other.font_loader),
            window_size: other.window_size,
            scaling: other.scaling,
            #[cfg(feature = "debug")]
            object_marker_texture: other.object_marker_texture.clone(),
            #[cfg(feature = "debug")]
            light_source_marker_texture: other.light_source_marker_texture.clone(),
            #[cfg(feature = "debug")]
            sound_source_marker_texture: other.sound_source_marker_texture.clone(),
            #[cfg(feature = "debug")]
            effect_source_marker_texture: other.effect_source_marker_texture.clone(),
            #[cfg(feature = "debug")]
            entity_marker_texture: other.entity_marker_texture.clone(),
            #[cfg(feature = "debug")]
            shadow_marker_texture: other.shadow_marker_texture.clone(),
        }
    }

    pub fn clear(&self) {
        self.instructions.borrow_mut().clear();
    }

    pub fn get_instructions(&self) -> Ref<Vec<RectangleInstruction>> {
        self.instructions.borrow()
    }

    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.window_size = window_size;
    }

    pub fn update_scaling(&mut self, scaling: Scaling) {
        self.scaling = scaling;
    }

    pub fn render_text(
        &self,
        text: &str,
        text_position: ScreenPosition,
        color: Color,
        font_size: FontSize,
        align_horizontal: AlignHorizontal,
    ) {
        let font_size = font_size.scaled(self.scaling);

        let TextLayout { glyphs, size } = self.font_loader.borrow_mut().get_text_layout(text, color, font_size, 1.0, f32::MAX);

        let horizontal_offset = match align_horizontal {
            AlignHorizontal::Left => 0.0,
            AlignHorizontal::Mid => -size.x / 2.0,
        };

        let mut instructions = self.instructions.borrow_mut();

        glyphs.iter().for_each(
            |GlyphInstruction {
                 position,
                 texture_coordinate,
                 color,
             }| {
                let screen_position = ScreenPosition {
                    left: text_position.left + position.min.x + horizontal_offset,
                    top: text_position.top + position.min.y,
                } / self.window_size;

                let screen_size = ScreenSize {
                    width: position.width(),
                    height: position.height(),
                } / self.window_size;

                let texture_position = texture_coordinate.min.to_vec();
                let texture_size = texture_coordinate.max - texture_coordinate.min;

                instructions.push(RectangleInstruction::Text {
                    screen_position,
                    screen_size,
                    color: *color,
                    texture_position,
                    texture_size,
                });
            },
        );
    }

    pub fn render_damage_text(&self, text: &str, position: ScreenPosition, color: Color, font_size: FontSize) {
        self.render_text(text, position, color, font_size, AlignHorizontal::Mid);
    }

    pub fn render_bar(&self, position: ScreenPosition, size: ScreenSize, color: Color, maximum: f32, current: f32) {
        let bar_offset = ScreenSize::only_width(size.width / 2.0);
        let bar_size = ScreenSize {
            width: (size.width / maximum) * current,
            height: size.height,
        };

        self.render_rectangle(position - bar_offset, bar_size, color);
    }

    pub fn render_rectangle(&self, position: ScreenPosition, size: ScreenSize, color: Color) {
        let screen_position = position / self.window_size;
        let screen_size = size / self.window_size;

        self.instructions.borrow_mut().push(RectangleInstruction::Solid {
            screen_position,
            screen_size,
            color,
        });
    }

    fn render_indexed(
        &self,
        texture: Arc<Texture>,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
        column_count: usize,
        cell_index: usize,
        smooth: bool,
    ) {
        let screen_position = ScreenPosition {
            left: screen_position.left / self.window_size.width,
            top: screen_position.top / self.window_size.height,
        };

        let screen_size = ScreenSize {
            width: screen_size.width / self.window_size.width,
            height: screen_size.height / self.window_size.height,
        };

        let unit = 1.0 / column_count as f32;
        let offset_x = unit * (cell_index % column_count) as f32;
        let offset_y = unit * (cell_index / column_count) as f32;
        let texture_position = Vector2::new(offset_x, offset_y);
        let texture_size = Vector2::new(unit, unit);

        self.instructions.borrow_mut().push(RectangleInstruction::Sprite {
            screen_position,
            screen_size,
            color,
            texture_position,
            texture_size,
            linear_filtering: smooth,
            texture,
        });
    }
}

#[cfg(feature = "debug")]
impl MarkerRenderer for GameInterfaceRenderer {
    fn render_marker(&mut self, camera: &dyn Camera, marker_identifier: MarkerIdentifier, position: Point3<f32>, hovered: bool) {
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, MarkerIdentifier::SIZE);

        if top_left_position.w >= 0.1 && bottom_right_position.w >= 0.1 {
            let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);

            let (texture, color) = match marker_identifier {
                MarkerIdentifier::Object(..) if hovered => (&self.object_marker_texture, Color::rgb_u8(235, 180, 52)),
                MarkerIdentifier::Object(..) => (&self.object_marker_texture, Color::rgb_u8(235, 103, 52)),
                MarkerIdentifier::LightSource(..) if hovered => (&self.light_source_marker_texture, Color::rgb_u8(150, 52, 235)),
                MarkerIdentifier::LightSource(..) => (&self.light_source_marker_texture, Color::rgb_u8(52, 235, 217)),
                MarkerIdentifier::SoundSource(..) if hovered => (&self.sound_source_marker_texture, Color::rgb_u8(128, 52, 235)),
                MarkerIdentifier::SoundSource(..) => (&self.sound_source_marker_texture, Color::rgb_u8(235, 52, 140)),
                MarkerIdentifier::EffectSource(..) if hovered => (&self.effect_source_marker_texture, Color::rgb_u8(235, 52, 52)),
                MarkerIdentifier::EffectSource(..) => (&self.effect_source_marker_texture, Color::rgb_u8(52, 235, 156)),
                MarkerIdentifier::Particle(..) if hovered => return,
                MarkerIdentifier::Particle(..) => return,
                MarkerIdentifier::Entity(..) if hovered => (&self.entity_marker_texture, Color::rgb_u8(235, 92, 52)),
                MarkerIdentifier::Entity(..) => (&self.entity_marker_texture, Color::rgb_u8(189, 235, 52)),
                MarkerIdentifier::Shadow(..) if hovered => (&self.shadow_marker_texture, Color::rgb_u8(200, 200, 200)),
                MarkerIdentifier::Shadow(..) => (&self.shadow_marker_texture, Color::rgb_u8(170, 170, 170)),
            };

            self.instructions.borrow_mut().push(RectangleInstruction::Sdf {
                screen_position,
                screen_size,
                color,
                texture_position: Vector2::new(0.0, 0.0),
                texture_size: Vector2::new(1.0, 1.0),
                texture: texture.clone(),
            });
        }
    }
}
