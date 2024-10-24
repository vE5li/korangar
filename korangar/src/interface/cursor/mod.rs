use std::sync::Arc;

use korangar_interface::application::ClipTraitExt;
use ragnarok_packets::ClientTick;

use super::application::InterfaceSettings;
use super::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::graphics::Color;
use crate::input::Grabbed;
use crate::loaders::{ActionLoader, Actions, AnimationState, Sprite, SpriteLoader};
use crate::renderer::{GameInterfaceRenderer, SpriteRenderer};

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum MouseCursorState {
    Default,
    Dialog,
    Click,
    Unsure0,
    RotateCamera,
    Attack,
    Attack1,
    Warp,
    NoAction,
    Grab,
    Unsure1,
    Unsure2,
    WarpFast,
    Unsure3,
}

pub struct MouseCursor {
    sprite: Arc<Sprite>,
    actions: Arc<Actions>,
    animation_state: AnimationState,
    shown: bool,
}

impl MouseCursor {
    pub fn new(sprite_loader: &mut SpriteLoader, action_loader: &mut ActionLoader) -> Self {
        let sprite = sprite_loader.get("cursors.spr").unwrap();
        let actions = action_loader.get("cursors.act").unwrap();
        let animation_state = AnimationState::new(ClientTick(0));
        let shown = true;

        Self {
            sprite,
            actions,
            animation_state,
            shown,
        }
    }

    pub fn hide(&mut self) {
        self.shown = false;
    }

    pub fn show(&mut self) {
        self.shown = true;
    }

    pub fn update(&mut self, client_tick: ClientTick) {
        self.animation_state.update(client_tick);
    }

    // TODO: this is just a workaround until i find a better solution to make the
    // cursor always look correct.
    pub fn set_start_time(&mut self, client_tick: ClientTick) {
        self.animation_state.start_time = client_tick;
    }

    pub fn set_state(&mut self, state: MouseCursorState, client_tick: ClientTick) {
        let new_state = state as usize;

        if self.animation_state.action != new_state {
            self.animation_state.start_time = client_tick;
        }

        self.animation_state.action = new_state;
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render mouse cursor"))]
    pub fn render(
        &self,
        renderer: &GameInterfaceRenderer,
        mouse_position: ScreenPosition,
        grabbed: Option<Grabbed>,
        color: Color,
        application: &InterfaceSettings,
    ) {
        if !self.shown {
            return;
        }

        if let Some(grabbed) = grabbed {
            match grabbed {
                Grabbed::Texture(texture) => renderer.render_sprite(
                    texture.clone(),
                    mouse_position - ScreenSize::uniform(15.0 * application.get_scaling_factor()),
                    ScreenSize::uniform(30.0 * application.get_scaling_factor()),
                    ScreenClip::unbound(),
                    Color::WHITE,
                    false,
                ),
                Grabbed::Action(sprite, actions, animation_state) => actions.render(
                    renderer,
                    &sprite,
                    &animation_state,
                    mouse_position,
                    0,
                    Color::WHITE,
                    application,
                ),
            }
        }

        // TODO: figure out how this is actually supposed to work
        let direction = match self.animation_state.action {
            0 | 2 | 4 => 0,
            _ => 7,
        };

        self.actions.render(
            renderer,
            &self.sprite,
            &self.animation_state,
            mouse_position,
            direction,
            color,
            application,
        );
    }
}
