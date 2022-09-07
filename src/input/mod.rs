mod event;
mod key;
mod mode;

use std::rc::Rc;

use cgmath::Vector2;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};

pub use self::event::UserEvent;
pub use self::key::Key;
pub use self::mode::MouseInputMode;
use crate::graphics::{PickerRenderTarget, PickerTarget, RenderSettings};
use crate::interface::{ClickAction, ElementCell, Interface, MouseCursorState};

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
    focused_element: Option<(ElementCell, usize)>,
    previous_focused_element: Option<(ElementCell, usize)>,
    input_buffer: Vec<char>,
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
        let focused_element = None;
        let previous_focused_element = None;
        let input_buffer = Vec::new();

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
            focused_element,
            previous_focused_element,
            input_buffer,
        }
    }

    pub fn reset(&mut self) {

        self.left_mouse_button.reset();
        self.right_mouse_button.reset();
        self.keys.iter_mut().for_each(|key| key.reset());
        self.mouse_input_mode = MouseInputMode::None;
        self.focused_element = None;
    }

    pub fn update_mouse_position(&mut self, position: PhysicalPosition<f64>) {
        self.new_mouse_position = Vector2::new(position.x as f32, position.y as f32);
    }

    pub fn update_mouse_buttons(&mut self, button: MouseButton, state: ElementState) {

        let pressed = matches!(state, ElementState::Pressed);

        match button {
            MouseButton::Left => self.left_mouse_button.set_down(pressed),
            MouseButton::Right => self.right_mouse_button.set_down(pressed),
            _ignored => {}
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

    pub fn buffer_character(&mut self, character: char) {
        self.input_buffer.push(character);
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

    pub fn user_events(
        &mut self,
        interface: &mut Interface,
        picker_target: &mut PickerRenderTarget,
        render_settings: &RenderSettings,
        window_size: Vector2<usize>,
        client_tick: u32,
    ) -> (Vec<UserEvent>, Option<ElementCell>, Option<ElementCell>, Option<PickerTarget>) {

        let mut events = Vec::new();
        let mut mouse_target = None;
        let (mut hovered_element, mut window_index) = interface.hovered_element(self.new_mouse_position);

        let shift_down = self.keys[42].down();

        #[cfg(feature = "debug")]
        let lock_actions = render_settings.use_debug_camera;
        #[cfg(not(feature = "debug"))]
        let lock_actions = false;

        if self.left_mouse_button.pressed() || self.right_mouse_button.pressed() {
            self.focused_element = None;
        }

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
        }

        if let Some(window_index) = &mut window_index && self.mouse_input_mode.is_none() {
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

                            ClickAction::FocusElement => self.focused_element = Some((hovered_element.clone(), *window_index)),

                            ClickAction::Event(event) => events.push(event),

                            ClickAction::MoveInterface => self.mouse_input_mode = MouseInputMode::MoveInterface(*window_index),

                            ClickAction::DragElement => {
                                self.mouse_input_mode = MouseInputMode::DragElement((hovered_element.clone(), *window_index))
                            }

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

        if self.right_mouse_button.down()
            && !self.right_mouse_button.pressed()
            && self.mouse_input_mode.is_none()
            && self.mouse_delta.x != 0.0
            && !lock_actions
        {
            self.mouse_input_mode = MouseInputMode::RotateCamera;
        }

        if !self.mouse_input_mode.is_none() || shift_down {
            hovered_element = None;
        }

        match &self.mouse_input_mode {

            MouseInputMode::DragElement((element, window_index)) => {

                if self.mouse_delta != Vector2::new(0.0, 0.0) {
                    interface.drag_element(element, *window_index, self.mouse_delta);
                }
                interface.set_mouse_cursor_state(MouseCursorState::Grab, client_tick);
            }

            MouseInputMode::MoveInterface(identifier) => {

                if self.mouse_delta != Vector2::new(0.0, 0.0) {
                    interface.move_window(*identifier, self.mouse_delta);
                }
                interface.set_mouse_cursor_state(MouseCursorState::Grab, client_tick);
            }

            MouseInputMode::ResizeInterface(identifier) => {
                if self.mouse_delta != Vector2::new(0.0, 0.0) {
                    interface.resize_window(*identifier, self.mouse_delta);
                }
            }

            MouseInputMode::RotateCamera => {

                events.push(UserEvent::CameraRotate(self.mouse_delta.x));
                interface.set_mouse_cursor_state(MouseCursorState::RotateCamera, client_tick);
            }

            MouseInputMode::ClickInterface => interface.set_mouse_cursor_state(MouseCursorState::Click, client_tick),

            MouseInputMode::None => {}
        }

        if self.scroll_delta != 0.0 {
            if let Some(window_index) = window_index {
                if let Some(element) = &hovered_element {
                    interface.scroll_element(element, window_index, self.scroll_delta);
                }
            } else if !lock_actions {
                events.push(UserEvent::CameraZoom(self.scroll_delta));
            }
        }

        let characters = self.input_buffer.drain(..);

        if let Some((focused_element, focused_window)) = &mut self.focused_element {

            if self.keys[15].pressed() {

                let new_focused_element = focused_element
                    .borrow()
                    .focus_next(focused_element.clone(), None, shift_down.into());

                if let Some(new_focused_element) = new_focused_element {
                    *focused_element = new_focused_element;
                }
            }

            if self.keys[28].pressed() {

                let action = interface.left_click_element(focused_element, *focused_window);

                if let Some(ClickAction::Event(event)) = &action {
                    println!("{:?}", event);
                }

                if let Some(action) = action {
                    // TODO: remove and replace with proper event
                    match action {
                        ClickAction::Event(event) => events.push(event),
                        ClickAction::OpenWindow(prototype_window) => interface.open_window(prototype_window.as_ref()),
                        ClickAction::CloseWindow => interface.close_window(*focused_window),
                        _ => panic!(),
                    }
                }
            }

            for character in characters {
                match character {
                    // ignore since we need to handle tab knowing the state of shift
                    '\t' => {}
                    valid => interface.input_character_element(focused_element, *focused_window, valid),
                }
            }
        } else {

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
            if self.right_mouse_button.down()
                && !self.right_mouse_button.pressed()
                && self.mouse_input_mode.is_none()
                && render_settings.use_debug_camera
            {
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
        }

        if window_index.is_none() && self.mouse_input_mode.is_none() {

            if let Some(fence) = picker_target.state.try_take_fence() {
                fence.wait(None).unwrap();
            }

            let sample_index = self.new_mouse_position.x as usize + self.new_mouse_position.y as usize * window_size.x;
            let lock = picker_target.buffer.read().unwrap();

            if sample_index < lock.len() {

                let pixel = lock[sample_index];

                if pixel != 0 {

                    let picker_target = PickerTarget::from(pixel);

                    if self.left_mouse_button.pressed() && self.mouse_input_mode.is_none() {
                        match picker_target {

                            PickerTarget::Tile(x, y) => events.push(UserEvent::RequestPlayerMove(Vector2::new(x as usize, y as usize))),

                            PickerTarget::Entity(entity_id) => events.push(UserEvent::RequestPlayerInteract(entity_id)),

                            #[cfg(feature = "debug")]
                            PickerTarget::Marker(marker_identifier) => events.push(UserEvent::OpenMarkerDetails(marker_identifier)),
                        }
                    }

                    mouse_target = Some(picker_target);
                }
            }
        }

        // TODO: this will fail if the user hovers over an entity that changes the cursor and then
        // immediately over a different one that doesn't, because main wont set the default cursor
        if self.mouse_input_mode.is_none() && !matches!(mouse_target, Some(PickerTarget::Entity(_))) {
            interface.set_mouse_cursor_state(MouseCursorState::Default, client_tick);
        }

        // to fix redrawing twice when clicking on elements
        if !self.mouse_input_mode.is_none() {
            hovered_element = None;
        }

        // check if the hovered element changed from last frame
        let rerender_hovered = self
            .previous_hovered_element
            .as_ref()
            .zip(hovered_element.as_ref())
            .map(|(previous, current)| !Rc::ptr_eq(&previous.0, current))
            .unwrap_or(self.previous_hovered_element.is_some() || hovered_element.is_some());

        if rerender_hovered {

            if let Some((_element, window_index)) = &self.previous_hovered_element {
                interface.schedule_rerender_window(*window_index);
            }

            if let Some(window_index) = window_index {
                interface.schedule_rerender_window(window_index);
            }
        }

        // check if the focused element changed from last frame
        let rerender_focused = self
            .previous_focused_element
            .as_ref()
            .zip(self.focused_element.as_ref())
            .map(|(previous, current)| !Rc::ptr_eq(&previous.0, &current.0))
            .unwrap_or(self.previous_focused_element.is_some() || self.focused_element.is_some());

        if rerender_focused {

            if let Some((_element, window_index)) = &self.previous_focused_element {
                interface.schedule_rerender_window(*window_index);
            }

            if let Some((_element, window_index)) = &self.focused_element {
                interface.schedule_rerender_window(*window_index);
            }
        }

        self.previous_hovered_element = hovered_element.clone().zip(window_index);
        self.previous_focused_element = self.focused_element.clone();

        (
            events,
            hovered_element,
            self.focused_element.clone().map(|(element, _)| element),
            mouse_target,
        )
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
