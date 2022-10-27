use std::sync::Arc;

use cgmath::{Array, Vector2};
use vulkano::sync::GpuFuture;

use super::InterfaceSettings;
use crate::graphics::{Color, DeferredRenderer, Renderer, Texture};
use crate::loaders::{ActionLoader, Actions, AnimationState, GameFileLoader, Sprite, SpriteLoader};

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
}

impl MouseCursor {
    pub fn new(
        game_file_loader: &mut GameFileLoader,
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        texture_future: &mut Box<dyn GpuFuture + 'static>,
    ) -> Self {
        let sprite = sprite_loader.get("cursors.spr", game_file_loader, texture_future).unwrap();
        let actions = action_loader.get("cursors.act", game_file_loader).unwrap();
        let animation_state = AnimationState::new(0);

        Self {
            sprite,
            actions,
            animation_state,
        }
    }

    pub fn update(&mut self, client_tick: u32) {
        self.animation_state.update(client_tick);
    }

    // TODO: this is just a workaround until i find a better solution to make the
    // cursor always look correct.
    pub fn set_start_time(&mut self, client_tick: u32) {
        self.animation_state.start_time = client_tick;
    }

    pub fn set_state(&mut self, state: MouseCursorState, client_tick: u32) {
        let new_state = state as usize;

        if self.animation_state.action != new_state {
            self.animation_state.start_time = client_tick;
        }

        self.animation_state.action = new_state;
    }

    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        mouse_position: Vector2<f32>,
        grabbed_item: Option<Texture>,
        color: Color,
        interface_settings: &InterfaceSettings,
    ) {
        if let Some(item) = grabbed_item {
            renderer.render_sprite(
                render_target,
                item,
                mouse_position - Vector2::from_value(15.0 * *interface_settings.scaling),
                Vector2::from_value(30.0 * *interface_settings.scaling),
                Color::monochrome(255),
            );
        }

        // TODO: figure out how this is actually supposed to work
        let direction = match self.animation_state.action {
            0 | 2 | 4 => 0,
            _ => 7,
        };

        self.actions.render2(
            render_target,
            renderer,
            &self.sprite,
            &self.animation_state,
            mouse_position,
            direction,
            color,
            interface_settings,
        );
    }
}
