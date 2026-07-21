use std::sync::Arc;

use korangar_interface::application::Clip;
use ragnarok_packets::{ClientTick, SkillLevel};

use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize};
use crate::input::Grabbed;
use crate::loaders::{ActionLoader, FontSize, Sprite, SpriteLoader};
use crate::renderer::{AlignHorizontal, GameInterfaceRenderer, SpriteRenderer};
use crate::world::{Actions, SpriteAnimationState};

const PICKUP_DURATION_MS: u32 = 150;

/// Number of actions in the classic cursor act.
const CURSOR_ACTION_COUNT: usize = 14;

/// The direction that makes `base * 8 + direction` reduce to `base` modulo
/// the cursor act's action count, so each cursor state reaches its own
/// action. See the comment at the call site for the arithmetic.
fn cursor_direction(action_base_offset: usize) -> usize {
    7 * (action_base_offset % 2)
}

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MouseCursorState {
    Default = 0,
    Dialog = 1,
    Click = 2,
    Unsure0 = 3,
    RotateCamera = 4,
    Attack = 5,
    Attack1 = 6,
    Warp = 7,
    NoAction = 8,
    Grab = 9,
    /// The spinning aim circle shown while a skill is armed for targeting.
    /// Action 10 in the cursor act, matching the reference client's TARGET.
    Target = 10,
    Unsure2 = 11,
    WarpFast = 12,
    Unsure3 = 13,
    /// Not an orignial state, represented as part of this enum to make the API
    /// more ergonomic. Will likely be refactored with this entire module at
    /// some point.
    HoverItem = 900,
    GrabResource,
    PickUpItem,
}

impl From<MouseCursorState> for usize {
    fn from(value: MouseCursorState) -> Self {
        value as usize
    }
}

pub struct MouseCursor {
    sprite: Arc<Sprite>,
    actions: Arc<Actions>,
    cursor_state: MouseCursorState,
    animation_state: SpriteAnimationState,
    locked_until: ClientTick,
    shown: bool,
}

impl MouseCursor {
    pub fn new(sprite_loader: &SpriteLoader, action_loader: &ActionLoader) -> Self {
        let sprite = sprite_loader.get_or_load("cursors.spr").unwrap();
        let actions = action_loader.get_or_load("cursors.act").unwrap();
        let animation_state = SpriteAnimationState::new(ClientTick(0));
        let locked_until = ClientTick(0);
        let shown = true;

        Self {
            sprite,
            actions,
            cursor_state: MouseCursorState::Default,
            animation_state,
            locked_until,
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

    pub fn set_state(&mut self, state: MouseCursorState, client_tick: ClientTick) {
        if client_tick.0 > self.locked_until.0 {
            // Cursor is unlocked
            if self.cursor_state != state {
                self.cursor_state = state;

                let base_offset = match state {
                    MouseCursorState::PickUpItem => {
                        // Lock the cursor.
                        self.locked_until = ClientTick(client_tick.0 + PICKUP_DURATION_MS);

                        usize::from(MouseCursorState::Grab)
                    }
                    MouseCursorState::GrabResource => usize::from(MouseCursorState::Grab),
                    MouseCursorState::HoverItem => usize::from(MouseCursorState::Grab),
                    regular => usize::from(regular),
                };

                self.animation_state.action_base_offset = base_offset;
                self.animation_state.start_time = client_tick;
            }
        } else if self.cursor_state == state {
            // Cursor is locked, but we can still extend the duration of the state.
            self.locked_until = ClientTick(client_tick.0 + PICKUP_DURATION_MS);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render mouse cursor"))]
    pub fn render(
        &self,
        renderer: &GameInterfaceRenderer,
        mouse_position: ScreenPosition,
        grabbed: Option<Grabbed>,
        armed_skill_level: Option<SkillLevel>,
        color: Color,
        scaling: f32,
    ) {
        if !self.shown {
            return;
        }

        // Adjust the position of the mouse cursor based on the interface scale. At 1.0
        // the cursos is in the perfect position but for everything else the
        // sprite drifts from the mouse position. This might be cause by how we
        // scale sprites, needs further investigation.
        //
        // Values picked by testing. Can this be derived somehow?
        let mouse_position = ScreenPosition {
            left: mouse_position.left + 10.0 * (scaling - 1.0),
            top: mouse_position.top + 14.0 * (scaling - 1.0),
        };

        if let Some(grabbed) = grabbed {
            match grabbed {
                Grabbed::Texture(texture) => renderer.render_sprite(
                    texture.clone(),
                    mouse_position - ScreenSize::uniform(15.0 * scaling),
                    ScreenSize::uniform(30.0 * scaling),
                    ScreenClip::unbound(),
                    Color::WHITE,
                    false,
                ),
                Grabbed::Action(sprite, actions, animation_state) => actions.render_sprite(
                    renderer,
                    &sprite,
                    &animation_state,
                    mouse_position,
                    0,
                    ScreenClip::unbound(),
                    Color::WHITE,
                    scaling,
                ),
            }
        }

        // The cursor act is not directional: it has 14 actions, one per
        // cursor type, while the action index is computed as base * 8 +
        // direction and reduced modulo the action count. Reaching action N
        // therefore needs 8N + d = N (mod 14), i.e. d = 7N (mod 14): zero
        // for even actions and seven for odd ones. The previous hardcoded
        // state list satisfied this by accident for the states it used and
        // silently showed the wrong cursor for the even-numbered Target,
        // NoAction and WarpFast.
        let direction = cursor_direction(self.animation_state.action_base_offset);

        // TODO: Is there some deeper logic here?
        const HOVER_ITEM_FRAME: usize = 0;
        const PICKUP_FRAME: usize = 2;

        let frame_index = match self.cursor_state {
            MouseCursorState::HoverItem => Some(HOVER_ITEM_FRAME),
            MouseCursorState::PickUpItem => Some(PICKUP_FRAME),
            MouseCursorState::GrabResource => Some(PICKUP_FRAME),
            _ => None,
        };

        if let Some(frame_index) = frame_index {
            self.actions.render_sprite_frame(
                renderer,
                &self.sprite,
                self.animation_state.get_action_index(direction),
                frame_index,
                mouse_position,
                ScreenClip::unbound(),
                color,
                scaling,
            );
        } else {
            self.actions.render_sprite(
                renderer,
                &self.sprite,
                &self.animation_state,
                mouse_position,
                direction,
                ScreenClip::unbound(),
                color,
                scaling,
            );
        }

        // The armed skill's cast level rides beside the aim circle, white
        // over a dark offset copy so it reads on any ground. Offsets match
        // the reference client's placement next to its target cursor.
        if self.cursor_state == MouseCursorState::Target
            && let Some(skill_level) = armed_skill_level
        {
            let text = skill_level.0.to_string();
            let text_position = ScreenPosition {
                left: mouse_position.left + 20.0 * scaling,
                top: mouse_position.top - 18.0 * scaling,
            };
            let shadow_position = ScreenPosition {
                left: text_position.left + 1.5,
                top: text_position.top + 1.5,
            };
            let font_size = FontSize(20.0 * scaling);

            renderer.render_text(
                &text,
                shadow_position,
                Color::rgba_u8(30, 30, 30, 220),
                font_size,
                AlignHorizontal::Left,
            );
            renderer.render_text(&text, text_position, Color::WHITE, font_size, AlignHorizontal::Left);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cursor_state_reaches_its_own_action() {
        // The action index is base * 8 + direction, reduced modulo the act's
        // action count by the renderer. Every state must land on its own
        // action, including the even-numbered ones the old hardcoded
        // direction list routed to the wrong cursor.
        for base in 0..CURSOR_ACTION_COUNT {
            let direction = cursor_direction(base);
            assert_eq!(
                (base * 8 + direction) % CURSOR_ACTION_COUNT,
                base,
                "cursor action {base} must render as itself"
            );
        }

        // The armed-skill aim circle in particular: action 10.
        let target = usize::from(MouseCursorState::Target);
        assert_eq!((target * 8 + cursor_direction(target)) % CURSOR_ACTION_COUNT, 10);
    }
}
