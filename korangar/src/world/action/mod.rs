use std::ops::Mul;

use cgmath::{Array, Vector2};
use derive_new::new;
use korangar_audio::SoundEffectKey;
use korangar_interface::elements::{ElementCell, PrototypeElement};
use korangar_util::container::Cacheable;
use ragnarok_formats::action::{Action, ActionsData};
use ragnarok_packets::ClientTick;

use crate::graphics::Color;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::Sprite;
use crate::renderer::SpriteRenderer;

#[derive(Clone, Debug, new)]
pub struct SpriteAnimationState {
    #[new(default)]
    pub action_base_offset: usize,
    pub start_time: ClientTick,
    #[new(default)]
    pub time: u32,
}

impl SpriteAnimationState {
    pub fn update(&mut self, client_tick: ClientTick) {
        self.time = client_tick.0.wrapping_sub(self.start_time.0);
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ActionEvent {
    /// Start playing a WAV sound file.
    Sound { key: SoundEffectKey },
    /// An attack event when the "flinch" animation is played.
    Attack,
    /// Start playing a WAV sound file.
    Unknown,
}

impl PrototypeElement<InterfaceSettings> for ActionEvent {
    fn to_element(&self, display: String) -> ElementCell<InterfaceSettings> {
        match self {
            Self::Sound { .. } => PrototypeElement::to_element(&"Sound", display),
            Self::Attack => PrototypeElement::to_element(&"Attack", display),
            Self::Unknown => PrototypeElement::to_element(&"Unknown", display),
        }
    }
}

#[derive(Debug, PrototypeElement)]
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
        color: Color,
        application: &InterfaceSettings,
    ) {
        let direction = camera_direction % 8;
        let animation_action = animation_state.action_base_offset * 8 + direction;
        let action = &self.actions[animation_action % self.actions.len()];
        let delay = self.delays[animation_action % self.delays.len()];
        let factor = delay * 50.0;

        // We must use f64 here, so that the microsecond u32 value of
        // `animation_state.time` can always be properly represented.
        let frame = (f64::from(animation_state.time) / f64::from(factor)) as usize;

        let motion = &action.motions[frame % action.motions.len()];

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
            let zoom = sprite_clip.zoom.unwrap_or(1.0) * application.get_scaling_factor();
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

            let screen_clip = ScreenClip {
                left: 0.0,
                top: 0.0,
                right: f32::MAX,
                bottom: f32::MAX,
            };

            renderer.render_sprite(texture.clone(), final_position, final_size, screen_clip, color, false);
        }
    }
}

impl Cacheable for Actions {
    fn size(&self) -> usize {
        size_of_val(&self.actions)
    }
}
