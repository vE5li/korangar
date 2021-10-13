mod event;
mod key;
mod mode;

use cgmath::Vector2;
use winit::event::{ MouseButton, ElementState, MouseScrollDelta };
use winit::dpi::PhysicalPosition;

use interface::Interface;

pub use self::event::UserEvent;

use self::mode::MouseInputMode;
use self::key::Key;

const MOUSE_SCOLL_MULTIPLIER: f32 = 30.0;
const KEY_COUNT: usize = 128;

pub struct InputSystem {
    previous_mouse_position: Vector2<f32>,
    new_mouse_position: Vector2<f32>,
    mouse_delta: Vector2<f32>,
    previous_scroll_position: f32,
    new_scroll_position: f32,
    scroll_delta: f32,
    left_mouse_button: Key,
    right_mouse_button: Key,
    keys: [Key; KEY_COUNT],
    mouse_input_mode: MouseInputMode,
}

impl InputSystem {

    pub fn new() -> Self {

        let previous_mouse_position = Vector2::new(0.0, 0.0);
        let new_mouse_position = Vector2::new(0.0, 0.0);
        let mouse_delta = Vector2::new(0.0, 0.0);

        let previous_scroll_position = 0.0;
        let new_scroll_position = 0.0;
        let scroll_delta = 0.0;

        let left_mouse_button = Key::new();
        let right_mouse_button = Key::new();
        let keys = [Key::new(); KEY_COUNT];

        let mouse_input_mode = MouseInputMode::None;

        return Self { previous_mouse_position, new_mouse_position, mouse_delta, previous_scroll_position, new_scroll_position, scroll_delta, left_mouse_button, right_mouse_button, keys, mouse_input_mode };
    }

    pub fn reset(&mut self) {
        self.left_mouse_button = Key::new();
        self.right_mouse_button = Key::new();
        self.keys.iter_mut().for_each(|key| *key = Key::new());
        self.mouse_input_mode = MouseInputMode::None;
    }

    pub fn update_mouse_position(&mut self, position: PhysicalPosition<f64>) {
        self.new_mouse_position = Vector2::new(position.x as f32, position.y as f32);
    }

    pub fn update_mouse_buttons(&mut self, button: MouseButton, state: ElementState) {
        let pressed = matches!(state, ElementState::Pressed);

        match button {
            MouseButton::Left => self.left_mouse_button.set_down(pressed),
            MouseButton::Right => self.right_mouse_button.set_down(pressed),
            _ignored => {},
        }
    }

    pub fn update_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(_x, y) => self.new_scroll_position += y as f32 * MOUSE_SCOLL_MULTIPLIER,
            MouseScrollDelta::PixelDelta(position) => self.new_scroll_position += position.y as f32,
        }
    }

    pub fn update_keyboard(&mut self, code: usize, state: ElementState) {
        let pressed = matches!(state, ElementState::Pressed);
        self.keys[code].set_down(pressed);
    }

    pub fn update_delta(&mut self) {

        self.mouse_delta = self.new_mouse_position - self.previous_mouse_position;
        self.previous_mouse_position = self.new_mouse_position;

        self.scroll_delta = self.new_scroll_position - self.previous_scroll_position;
        self.previous_scroll_position = self.new_scroll_position;

        self.left_mouse_button.update();
        self.right_mouse_button.update();
        self.keys.iter_mut().for_each(|key| key.update());
    }

    pub fn user_events(&mut self, interface: &Interface) -> (Vec<UserEvent>, usize) {

        let mut events = Vec::new();

        let element = interface.hovered_element(self.new_mouse_position);

        if let Some(element) = element {
            if self.left_mouse_button.pressed() {

                if let Some(clickable) = element.clickable() {
                    events.push(clickable.click());
                    self.mouse_input_mode = MouseInputMode::Click;
                }

                if let Some(draggable) = element.draggable() {
                    let identifier = draggable.get_identifier();
                    self.mouse_input_mode = MouseInputMode::MoveInterface(identifier);
                }
            }
        }

        if self.left_mouse_button.released() {
            self.mouse_input_mode = MouseInputMode::None;
        }

        if let MouseInputMode::MoveInterface(identifier) = &self.mouse_input_mode {
            if self.mouse_delta != Vector2::new(0.0, 0.0) {
                events.push(UserEvent::MoveInterface(*identifier, self.mouse_delta));
            }
        }

        if self.scroll_delta != 0.0 {
            events.push(UserEvent::CameraZoom(-self.scroll_delta));
        }

        if self.right_mouse_button.down() && self.mouse_delta.x != 0.0 {
            events.push(UserEvent::CameraRotate(self.mouse_delta.x));
        }

        if self.keys[46].pressed() {
            events.push(UserEvent::ToggleShowFramesPerSecond);
        }

        #[cfg(feature = "debug")]
        if self.keys[33].pressed() {
            events.push(UserEvent::ToggleUseDebugCamera);
        }

        #[cfg(feature = "debug")]
        if self.left_mouse_button.down() && self.mouse_input_mode.is_none() {
            events.push(UserEvent::CameraLookAround(-self.mouse_delta));
        }

        #[cfg(feature = "debug")]
        if self.keys[17].down() {
            events.push(UserEvent::CameraMoveForward);
        }

        #[cfg(feature = "debug")]
        if self.keys[31].down() {
            events.push(UserEvent::CameraMoveBackward);
        }

        #[cfg(feature = "debug")]
        if self.keys[30].down() {
            events.push(UserEvent::CameraMoveLeft);
        }

        #[cfg(feature = "debug")]
        if self.keys[32].down() {
            events.push(UserEvent::CameraMoveRight);
        }

        #[cfg(feature = "debug")]
        if self.keys[57].down() {
            events.push(UserEvent::CameraMoveUp);
        }

        #[cfg(feature = "debug")]
        if self.keys[42].down() {
            events.push(UserEvent::CameraMoveDown);
        }

        let element_index = element.map(|element| element.index()).unwrap_or(255); // usize.MAX
        return (events, element_index);
    }
}
