use cgmath::Vector2;
use winit::event::{ MouseButton, ElementState, MouseScrollDelta };
use winit::dpi::PhysicalPosition;

pub enum InputEvent {
    CameraZoom(f32),
    CameraRotate(f32),
    ToggleFramesPerSecond,
    #[cfg(feature = "debug")]
    ToggleDebugCamera,
    #[cfg(feature = "debug")]
    CameraLookAround(Vector2<f32>),
    #[cfg(feature = "debug")]
    CameraMoveForward,
    #[cfg(feature = "debug")]
    CameraMoveBackward,
    #[cfg(feature = "debug")]
    CameraMoveLeft,
    #[cfg(feature = "debug")]
    CameraMoveRight,
    #[cfg(feature = "debug")]
    CameraMoveUp,
    #[cfg(feature = "debug")]
    CameraMoveDown,
}

#[derive(Copy, Clone, Debug)]
pub struct Key {
    is_down: bool,
    was_down: bool,
    is_pressed: bool,
    is_released: bool,
}

impl Key {

    pub fn new() -> Self {

        let is_down = false;
        let was_down = false;
        let is_pressed = false;
        let is_released = false;

        return Self { is_down, was_down, is_pressed, is_released };
    }

    pub fn set_down(&mut self, is_down: bool) {
        self.is_down = is_down;
    }

    pub fn update(&mut self) {
        self.is_pressed = self.is_down && !self.was_down;
        self.is_released = !self.is_down && self.was_down;
        self.was_down = self.is_down;
    }

    pub fn down(&self) -> bool {
        return self.is_down;
    }

    pub fn pressed(&self) -> bool {
        return self.is_pressed;
    }
}

const MOUSE_SCOLL_MULTIPLIER: f32 = 30.0;
const KEY_COUNT: usize = 128;

pub struct InputSystem {
    previous_mouse_position: Vector2<f32>,
    new_mouse_position: Vector2<f32>,
    mouse_delta: Vector2<f32>,
    previous_scroll_position: f32,
    new_scroll_position: f32,
    scroll_delta: f32,
    left_mouse_button_pressed: bool,
    right_mouse_button_pressed: bool,
    keys: [Key; KEY_COUNT],
}

impl InputSystem {

    pub fn new() -> Self {

        let previous_mouse_position = Vector2::new(0.0, 0.0);
        let new_mouse_position = Vector2::new(0.0, 0.0);
        let mouse_delta = Vector2::new(0.0, 0.0);

        let previous_scroll_position = 0.0;
        let new_scroll_position = 0.0;
        let scroll_delta = 0.0;

        let left_mouse_button_pressed = false;
        let right_mouse_button_pressed = false;

        let keys = [Key::new(); KEY_COUNT];

        return Self { previous_mouse_position, new_mouse_position, mouse_delta, previous_scroll_position, new_scroll_position, scroll_delta, left_mouse_button_pressed, right_mouse_button_pressed, keys };
    }

    pub fn reset(&mut self) {

        self.left_mouse_button_pressed = false;
        self.right_mouse_button_pressed = false;

        self.keys.iter_mut().for_each(|key| *key = Key::new());
    }

    pub fn update_mouse_position(&mut self, position: PhysicalPosition<f64>) {
        self.new_mouse_position = Vector2::new(position.x as f32, position.y as f32);
    }

    pub fn update_mouse_buttons(&mut self, button: MouseButton, state: ElementState) {
        let pressed = matches!(state, ElementState::Pressed);

        match button {
            MouseButton::Left => self.left_mouse_button_pressed = pressed,
            MouseButton::Right => self.right_mouse_button_pressed = pressed,
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

        self.mouse_delta = self.previous_mouse_position - self.new_mouse_position;
        self.previous_mouse_position = self.new_mouse_position;

        self.scroll_delta = self.previous_scroll_position - self.new_scroll_position;
        self.previous_scroll_position = self.new_scroll_position;

        self.keys.iter_mut().for_each(|key| key.update());
    }

    pub fn input_events(&self) -> Vec<InputEvent> {
        let mut events = Vec::new();

        if self.scroll_delta != 0.0 {
            events.push(InputEvent::CameraZoom(self.scroll_delta));
        }

        if self.right_mouse_button_pressed && self.mouse_delta.x != 0.0 {
            events.push(InputEvent::CameraRotate(-self.mouse_delta.x));
        }

        if self.keys[46].pressed() {
            events.push(InputEvent::ToggleFramesPerSecond);
        }

        #[cfg(feature = "debug")]
        if self.keys[33].pressed() {
            events.push(InputEvent::ToggleDebugCamera);
        }

        #[cfg(feature = "debug")]
        if self.left_mouse_button_pressed {
            events.push(InputEvent::CameraLookAround(self.mouse_delta));
        }

        #[cfg(feature = "debug")]
        if self.keys[17].down() {
            events.push(InputEvent::CameraMoveForward);
        }

        #[cfg(feature = "debug")]
        if self.keys[31].down() {
            events.push(InputEvent::CameraMoveBackward);
        }

        #[cfg(feature = "debug")]
        if self.keys[30].down() {
            events.push(InputEvent::CameraMoveLeft);
        }

        #[cfg(feature = "debug")]
        if self.keys[32].down() {
            events.push(InputEvent::CameraMoveRight);
        }

        #[cfg(feature = "debug")]
        if self.keys[57].down() {
            events.push(InputEvent::CameraMoveUp);
        }

        #[cfg(feature = "debug")]
        if self.keys[42].down() {
            events.push(InputEvent::CameraMoveDown);
        }

        return events;
    }
}
