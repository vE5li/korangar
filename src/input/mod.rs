mod event;
mod key;
mod mode;

use std::rc::Rc;
use cgmath::Vector2;

use winit::event::{ MouseButton, ElementState, MouseScrollDelta };
use winit::dpi::PhysicalPosition;

use interface::types::ElementCell;
use interface::{ Interface, ClickAction };
use crate::graphics::{Renderer, RenderSettings};

pub use self::event::UserEvent;
pub use self::mode::MouseInputMode;
pub use self::key::Key;

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
    previous_hovered_element: Option<(ElementCell, usize)>,
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
        let previous_hovered_element = None;

        Self {
            previous_mouse_position,
            new_mouse_position,
            mouse_delta,
            previous_scroll_position,
            new_scroll_position,
            scroll_delta,
            left_mouse_button,
            right_mouse_button,
            keys,
            mouse_input_mode,
            previous_hovered_element,
        }
    }

    pub fn reset(&mut self) {
        self.left_mouse_button.reset();
        self.right_mouse_button.reset();
        self.keys.iter_mut().for_each(|key| key.reset());
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
        //println!("code: {}", code);
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

    pub fn user_events(&mut self, renderer: &mut Renderer, interface: &mut Interface, render_settings: &RenderSettings) -> (Vec<UserEvent>, Option<ElementCell>) {

        let mut events = Vec::new();
        let (mut hovered_element, mut window_index) = match self.mouse_input_mode.is_none() {         
            true => interface.hovered_element(self.new_mouse_position),
            false => (None, None),
        };

        let shift_down = self.keys[42].down();
  
        #[cfg(feature = "debug")]
        let lock_actions = render_settings.use_debug_camera;
        #[cfg(not(feature = "debug"))]
        let lock_actions = false;

        if shift_down {

            if let Some(window_index) = &mut window_index {

                if self.left_mouse_button.pressed() {
                    *window_index = interface.move_window_to_top(*window_index);
                    self.mouse_input_mode = MouseInputMode::MoveInterface(*window_index);
                }

                if self.right_mouse_button.pressed() {
                    *window_index = interface.move_window_to_top(*window_index);
                    self.mouse_input_mode = MouseInputMode::ResizeInterface(*window_index);
                }
            }
            
            hovered_element = None;
        }

        if let Some(window_index) = &mut window_index {
            if (self.left_mouse_button.pressed() || self.right_mouse_button.pressed()) && !shift_down {

                *window_index = interface.move_window_to_top(*window_index);
                self.mouse_input_mode = MouseInputMode::ClickInterface;

                if let Some(hovered_element) = &hovered_element {

                    let action = match self.left_mouse_button.pressed() {
                        true => interface.left_click_element(hovered_element, *window_index),
                        false => interface.right_click_element(hovered_element, *window_index),
                    };

                    if let Some(action) = action {
                        match action {
                            ClickAction::Event(event) => events.push(event),
                            ClickAction::MoveInterface => self.mouse_input_mode = MouseInputMode::MoveInterface(*window_index),
                            ClickAction::DragElement => self.mouse_input_mode = MouseInputMode::DragElement((Rc::clone(hovered_element), *window_index)),
                            ClickAction::OpenWindow(prototype_window) => interface.open_window(prototype_window.as_ref()),
                            ClickAction::CloseWindow => interface.close_window(*window_index),
                        }
                    }
                }
            }
        }

        if self.left_mouse_button.released() {
            if let MouseInputMode::MoveInterface(identifier) = self.mouse_input_mode {
                match self.right_mouse_button.down() && !self.right_mouse_button.released() {
                    true => self.mouse_input_mode = MouseInputMode::ResizeInterface(identifier),
                    false => self.mouse_input_mode = MouseInputMode::None,
                }
            } else {
                self.mouse_input_mode = MouseInputMode::None;
            }
        }

        if self.right_mouse_button.released() {
            if let MouseInputMode::ResizeInterface(identifier) = self.mouse_input_mode {
                match self.left_mouse_button.down() && !self.left_mouse_button.released() {
                    true => self.mouse_input_mode = MouseInputMode::MoveInterface(identifier),
                    false => self.mouse_input_mode = MouseInputMode::None,
                }
            } else {
                self.mouse_input_mode = MouseInputMode::None;
            }
        }

        if let MouseInputMode::DragElement((element, window_index)) = &self.mouse_input_mode {
            if self.mouse_delta != Vector2::new(0.0, 0.0) {
                interface.drag_element(element, *window_index, self.mouse_delta);
            }
        }

        if let MouseInputMode::MoveInterface(identifier) = &self.mouse_input_mode {
            if self.mouse_delta != Vector2::new(0.0, 0.0) {
                interface.move_window(*identifier, self.mouse_delta);
            }
        }

        if let MouseInputMode::ResizeInterface(identifier) = &self.mouse_input_mode {
            if self.mouse_delta != Vector2::new(0.0, 0.0) {
                interface.resize_window(*identifier, self.mouse_delta);
            }
        }

        if self.right_mouse_button.down() && !self.right_mouse_button.pressed() && self.mouse_input_mode.is_none() && self.mouse_delta.x != 0.0 && !lock_actions {
            events.push(UserEvent::CameraRotate(self.mouse_delta.x));
        }

        if self.scroll_delta != 0.0 {
            if let Some(_window_index) = window_index {
                // TODO: scroll window
            } else if !lock_actions {
                events.push(UserEvent::CameraZoom(-self.scroll_delta));
            }
        }

        if self.left_mouse_button.pressed() && self.mouse_input_mode.is_none() && !lock_actions {
            let window_size = renderer.get_window_size();
            let picker_buffer = renderer.get_picker_buffer();
            let pixel = picker_buffer.read().unwrap()[self.new_mouse_position.x as usize + self.new_mouse_position.y as usize * window_size.x];

            if pixel != 0 {
                let x = (pixel & 0xff) - 1;
                let y = (pixel >> 16) - 1;
                events.push(UserEvent::RequestPlayerMove(Vector2::new(x as usize, y as usize)));
            }
        }

        if self.keys[1].pressed() {
            events.push(UserEvent::OpenMenuWindow);
        }

        #[cfg(feature = "debug")]
        if self.keys[50].pressed() {
            events.push(UserEvent::OpenMapsWindow);
        }

        #[cfg(feature = "debug")]
        if self.keys[19].pressed() {
            events.push(UserEvent::OpenRenderSettingsWindow);
        }

        #[cfg(feature = "debug")]
        if self.keys[42].pressed() && render_settings.use_debug_camera {
            events.push(UserEvent::CameraAccelerate);
        }

        #[cfg(feature = "debug")]
        if self.keys[42].released() && render_settings.use_debug_camera {
            events.push(UserEvent::CameraDecelerate);
        }

        #[cfg(feature = "debug")]
        if self.keys[33].pressed() {
            events.push(UserEvent::ToggleUseDebugCamera);
            events.push(UserEvent::CameraDecelerate);
        }

        #[cfg(feature = "debug")]
        if self.left_mouse_button.down() && !self.left_mouse_button.pressed() && self.mouse_input_mode.is_none() && render_settings.use_debug_camera {
            events.push(UserEvent::CameraLookAround(-self.mouse_delta));
        }

        #[cfg(feature = "debug")]
        if self.keys[17].down() && render_settings.use_debug_camera {
            events.push(UserEvent::CameraMoveForward);
        }

        #[cfg(feature = "debug")]
        if self.keys[31].down() && render_settings.use_debug_camera {
            events.push(UserEvent::CameraMoveBackward);
        }

        #[cfg(feature = "debug")]
        if self.keys[30].down() && render_settings.use_debug_camera {
            events.push(UserEvent::CameraMoveLeft);
        }

        #[cfg(feature = "debug")]
        if self.keys[32].down() && render_settings.use_debug_camera {
            events.push(UserEvent::CameraMoveRight);
        }

        #[cfg(feature = "debug")]
        if self.keys[57].down() && render_settings.use_debug_camera {
            events.push(UserEvent::CameraMoveUp);
        }

        // to fix redrawing twice when clicking on elements
        if !self.mouse_input_mode.is_none() {
            hovered_element = None;
        }

        let rerender = self.previous_hovered_element
            .as_ref()
            .zip(hovered_element.as_ref())
            .map(|(previous, current)| !Rc::ptr_eq(&previous.0, current))
            .unwrap_or(self.previous_hovered_element.is_some() || hovered_element.is_some());

        if rerender {

            if let Some((_element, window_index)) = &self.previous_hovered_element {
                interface.schedule_rerender_window(*window_index);
            }
            
            if let Some(window_index) = window_index {
                interface.schedule_rerender_window(window_index);
            }
        }

        self.previous_hovered_element = hovered_element.clone().zip(window_index);

        (events, hovered_element)
    }
    
    pub fn unused_left_click(&self) -> bool {
        self.left_mouse_button.pressed() && self.mouse_input_mode.is_none()
    }
    
    pub fn set_interface_clicked(&mut self) {
        self.mouse_input_mode = MouseInputMode::ClickInterface;
    }

    pub fn mouse_position(&self) -> Vector2<f32> {
        self.new_mouse_position
    }
}
