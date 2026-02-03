use std::ops::Mul;

use cgmath::{Array, Vector2};
use korangar_audio::SoundEffectKey;
use korangar_container::Cacheable;
use korangar_interface::element::StateElement;
use ragnarok_formats::action::Action;
#[cfg(feature = "debug")]
use ragnarok_formats::action::ActionsData;
use ragnarok_packets::ClientTick;
use rust_state::RustState;

use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::Sprite;
use crate::renderer::SpriteRenderer;

#[derive(Clone, Debug, RustState, StateElement)]
pub struct SpriteAnimationState {
    pub action_base_offset: usize,
    pub start_time: ClientTick,
    pub time: u32,
}

impl SpriteAnimationState {
    pub fn new(start_time: ClientTick) -> Self {
        Self {
            action_base_offset: 0,
            start_time,
            time: 0,
        }
    }

    pub fn get_action_index(&self, direction: usize) -> usize {
        self.action_base_offset * 8 + direction
    }
}

impl SpriteAnimationState {
    pub fn update(&mut self, client_tick: ClientTick) {
        self.time = client_tick.0.wrapping_sub(self.start_time.0);
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, RustState, StateElement)]
pub enum ActionEvent {
    /// Start playing a WAV sound file.
    Sound { key: SoundEffectKey },
    /// An attack event when the "flinch" animation is played.
    Attack,
    /// Start playing a WAV sound file.
    Unknown,
}

#[derive(Debug, RustState, StateElement)]
pub struct Actions {
    pub actions: Vec<Action>,
    pub delays: Vec<f32>,
    #[hidden_element]
    pub events: Vec<ActionEvent>,
    #[cfg(feature = "debug")]
    pub actions_data: ActionsData,
}

impl Actions {
    pub fn render_sprite(
        &self,
        renderer: &impl SpriteRenderer,
        sprite: &Sprite,
        animation_state: &SpriteAnimationState,
        position: ScreenPosition,
        camera_direction: usize,
        screen_clip: ScreenClip,
        color: Color,
        scaling: f32,
    ) {
        let direction = camera_direction % 8;
        let action_index = animation_state.get_action_index(direction);
        let delay = self.delays[action_index % self.delays.len()];
        let factor = delay * 50.0;

        // We must use f64 here, so that the microsecond u32 value of
        // `animation_state.time` can always be properly represented.
        let frame = (f64::from(animation_state.time) / f64::from(factor)) as usize;

        self.render_sprite_frame(renderer, sprite, action_index, frame, position, screen_clip, color, scaling);
    }

    pub fn render_sprite_frame(
        &self,
        renderer: &impl SpriteRenderer,
        sprite: &Sprite,
        action_index: usize,
        frame_index: usize,
        position: ScreenPosition,
        screen_clip: ScreenClip,
        color: Color,
        scaling: f32,
    ) {
        let action = &self.actions[action_index % self.actions.len()];
        let motion = &action.motions[frame_index % action.motions.len()];

        for sprite_clip in &motion.sprite_clips {
            // `get` instead of a direct index in case a fallback was loaded
            let Some(texture) = sprite.textures.get(sprite_clip.sprite_number as usize) else {
                return;
            };

            let offset = sprite_clip.position.map(|component| component as f32);
            let dimensions = sprite_clip
                .size
                .unwrap_or_else(|| {
                    let image_size = texture.get_size();
                    Vector2::new(image_size.width, image_size.height)
                })
                .map(|component| component as f32);
            let zoom = sprite_clip.zoom.unwrap_or(1.0) * scaling;
            let zoom2 = sprite_clip.zoom2.unwrap_or_else(|| Vector2::from_value(1.0));

            let final_size = dimensions.zip(zoom2, f32::mul) * zoom;
            let final_position = Vector2::new(position.left, position.top) + offset - final_size / 2.0;

            let final_size = ScreenSize {
                width: final_size.x,
                height: final_size.y,
            };

            let final_position = ScreenPosition {
                left: final_position.x,
                top: final_position.y,
            };

            renderer.render_sprite(texture.clone(), final_position, final_size, screen_clip, color, false);
        }
    }
}

impl Cacheable for Actions {
    fn size(&self) -> usize {
        // We cache actions only by count.
        0
    }
}
