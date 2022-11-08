mod event;
mod key;
mod mode;

use std::mem::variant_count;
use std::rc::{Rc, Weak};

use cgmath::Vector2;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, VirtualKeyCode};

pub use self::event::UserEvent;
pub use self::key::Key;
pub use self::mode::MouseInputMode;
#[cfg(feature = "debug")]
use crate::graphics::RenderSettings;
use crate::graphics::{PickerRenderTarget, PickerTarget};
use crate::interface::{ClickAction, ElementCell, Focus, Interface, MouseCursorState, WeakElementCell};

const MOUSE_SCOLL_MULTIPLIER: f32 = 30.0;
const KEY_COUNT: usize = variant_count::<VirtualKeyCode>();

#[derive(Default)]
pub struct FocusState {
    focused_element: Option<(WeakElementCell, usize)>,
    previous_hovered_element: Option<(WeakElementCell, usize)>,
    previous_focused_element: Option<(WeakElementCell, usize)>,
}

impl FocusState {
    pub fn remove_focus(&mut self) {
        self.focused_element = None;
    }

    pub fn set_focused_element(&mut self, element: Option<ElementCell>, window_index: usize) {
        self.focused_element = element.as_ref().map(Rc::downgrade).zip(Some(window_index));
    }

    pub fn update_focused_element(&mut self, element: Option<ElementCell>, window_index: usize) {
        if let Some(element) = element {
            self.focused_element = Some((Rc::downgrade(&element), window_index));
        }
    }

    pub fn get_focused_element(&self) -> Option<(ElementCell, usize)> {
        let (element, index) = self.focused_element.clone().unzip();
        element.as_ref().and_then(Weak::upgrade).zip(index)
    }

    pub fn did_hovered_element_change(&self, hovered_element: &Option<ElementCell>) -> bool {
        self.previous_hovered_element
            .as_ref()
            .zip(hovered_element.as_ref())
            .map(|(previous, current)| !Weak::ptr_eq(&previous.0, &Rc::downgrade(current)))
            .unwrap_or(self.previous_hovered_element.is_some() || hovered_element.is_some())
    }

    pub fn did_focused_element_change(&self) -> bool {
        self.previous_focused_element
            .as_ref()
            .zip(self.focused_element.as_ref())
            .map(|(previous, current)| !Weak::ptr_eq(&previous.0, &current.0))
            .unwrap_or(self.previous_focused_element.is_some() || self.focused_element.is_some())
    }

    pub fn previous_hovered_window(&self) -> Option<usize> {
        self.previous_hovered_element.as_ref().map(|(_, index)| index).cloned()
    }

    pub fn focused_window(&self) -> Option<usize> {
        self.focused_element.as_ref().map(|(_, index)| index).cloned()
    }

    pub fn previous_focused_window(&self) -> Option<usize> {
        self.previous_focused_element.as_ref().map(|(_, index)| index).cloned()
    }

    pub fn update(&mut self, hovered_element: &Option<ElementCell>, window_index: Option<usize>) -> Option<ElementCell> {
        self.previous_hovered_element = hovered_element.as_ref().map(Rc::downgrade).zip(window_index);
        self.previous_focused_element = self.focused_element.clone();

        self.focused_element.clone().and_then(|(weak_element, _)| weak_element.upgrade())
    }
}

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

        let left_mouse_button = Key::default();
        let right_mouse_button = Key::default();
        let keys = [Key::default(); KEY_COUNT];

        let mouse_input_mode = MouseInputMode::None;
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
            input_buffer,
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
            _ignored => {}
        }
    }

    pub fn update_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        match delta {
            MouseScrollDelta::LineDelta(_x, y) => self.new_scroll_position += y * MOUSE_SCOLL_MULTIPLIER,
            MouseScrollDelta::PixelDelta(position) => self.new_scroll_position += position.y as f32,
        }
    }

    pub fn update_keyboard(&mut self, virtual_code: VirtualKeyCode, state: ElementState) {
        let pressed = matches!(state, ElementState::Pressed);
        self.keys[virtual_code as usize].set_down(pressed);
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

    fn get_key(&self, key_code: VirtualKeyCode) -> &Key {
        &self.keys[key_code as usize]
    }

    pub fn user_events(
        &mut self,
        interface: &mut Interface,
        focus_state: &mut FocusState,
        picker_target: &mut PickerRenderTarget,
        #[cfg(feature = "debug")] render_settings: &RenderSettings,
        window_size: Vector2<usize>,
        client_tick: u32,
    ) -> (Vec<UserEvent>, Option<ElementCell>, Option<ElementCell>, Option<PickerTarget>) {
        let mut events = Vec::new();
        let mut mouse_target = None;
        let (hovered_element, mut window_index) = interface.hovered_element(self.new_mouse_position, &self.mouse_input_mode);

        let shift_down = self.get_key(VirtualKeyCode::LShift).down();

        #[cfg(feature = "debug")]
        let lock_actions = render_settings.use_debug_camera;
        #[cfg(not(feature = "debug"))]
        let lock_actions = false;

        if self.left_mouse_button.pressed() || self.right_mouse_button.pressed() {
            focus_state.remove_focus();
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

        let condition = (self.left_mouse_button.pressed() || self.right_mouse_button.pressed()) && !shift_down;
        if let Some(window_index) = &mut window_index && self.mouse_input_mode.is_none() && condition {

            *window_index = interface.move_window_to_top(*window_index);
            self.mouse_input_mode = MouseInputMode::ClickInterface;

            if let Some(hovered_element) = &hovered_element {

                let action = match self.left_mouse_button.pressed() {
                    true => interface.left_click_element(hovered_element, *window_index),
                    false => interface.right_click_element(hovered_element, *window_index),
                };

                if let Some(action) = action {
                    match action {

                        ClickAction::ChangeEvent(..) => {}

                        ClickAction::FocusElement => {

                            let element_cell = hovered_element.clone();
                            let new_focused_element = hovered_element.borrow().focus_next(element_cell, None, Focus::downwards()); // TODO: check
                            focus_state.set_focused_element(new_focused_element, *window_index);
                        },

                        ClickAction::FocusNext(focus_mode) => {

                            let element_cell = hovered_element.clone();
                            let new_focused_element = hovered_element.borrow().focus_next(element_cell, None, Focus::new(focus_mode));
                            focus_state.update_focused_element(new_focused_element, *window_index);
                        },

                        ClickAction::Event(event) => events.push(event),

                        ClickAction::MoveInterface => self.mouse_input_mode = MouseInputMode::MoveInterface(*window_index),

                        ClickAction::DragElement => {
                            self.mouse_input_mode = MouseInputMode::DragElement((hovered_element.clone(), *window_index))
                        }

                        ClickAction::MoveItem(item_source, item) => {
                            self.mouse_input_mode = MouseInputMode::MoveItem(item_source, item);
                            // Needs to rerender because some elements will render differently
                            // based on the mouse input mode.
                            interface.schedule_rerender();
                        },

                        ClickAction::OpenWindow(prototype_window) => interface.open_window(focus_state, prototype_window.as_ref()),

                        ClickAction::CloseWindow => interface.close_window(focus_state, *window_index),
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
                let mouse_input_mode = std::mem::take(&mut self.mouse_input_mode);
                // Needs to rerender because some elements will render differently
                // based on the mouse input mode.
                interface.schedule_rerender();

                if let MouseInputMode::MoveItem(item_source, item) = mouse_input_mode {
                    if let Some(hovered_element) = &hovered_element {
                        if let Some(item_move) = hovered_element.borrow_mut().drop_item(item_source, item) {
                            events.push(UserEvent::MoveItem(item_move));
                        }
                    }
                }
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
            MouseInputMode::MoveItem(..) | MouseInputMode::MoveSkill(..) => {}
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

        let characters = self.input_buffer.drain(..).collect::<Vec<_>>();

        if let Some((focused_element, focused_window)) = &focus_state.get_focused_element() {
            // this will currently not affect the following statements, which is a bit
            // strange
            if self.get_key(VirtualKeyCode::Escape).pressed() {
                focus_state.remove_focus();
            }

            if self.get_key(VirtualKeyCode::Tab).pressed() {
                let new_focused_element = focused_element
                    .borrow()
                    .focus_next(focused_element.clone(), None, Focus::new(shift_down.into()));

                focus_state.update_focused_element(new_focused_element, *focused_window);
            }

            if self.get_key(VirtualKeyCode::Return).pressed() {
                let action = interface.left_click_element(focused_element, *focused_window);

                if let Some(action) = action {
                    // TODO: remove and replace with proper event
                    match action {
                        ClickAction::Event(event) => events.push(event),
                        ClickAction::OpenWindow(prototype_window) => interface.open_window(focus_state, prototype_window.as_ref()),
                        ClickAction::CloseWindow => interface.close_window(focus_state, *focused_window),
                        _ => {}
                    }
                }
            }
        }

        if let Some((focused_element, focused_window)) = &focus_state.get_focused_element() {
            for character in characters {
                match character {
                    // ignore since we need to handle tab knowing the state of shift
                    '\t' => {}
                    '\x1b' => {}
                    valid => {
                        if let Some(action) = interface.input_character_element(focused_element, *focused_window, valid) {
                            match action {
                                // is handled in the interface
                                ClickAction::ChangeEvent(..) => {}
                                ClickAction::FocusElement => {
                                    let element_cell = focused_element.clone();
                                    let new_focused_element = focused_element.borrow().focus_next(element_cell, None, Focus::downwards());

                                    focus_state.set_focused_element(new_focused_element, *focused_window);
                                }
                                ClickAction::FocusNext(focus_mode) => {
                                    let element_cell = focused_element.clone();
                                    let new_focused_element =
                                        focused_element.borrow().focus_next(element_cell, None, Focus::new(focus_mode));

                                    focus_state.update_focused_element(new_focused_element, *focused_window);
                                }
                                ClickAction::Event(event) => events.push(event),
                                ClickAction::MoveInterface => self.mouse_input_mode = MouseInputMode::MoveInterface(*focused_window),
                                ClickAction::DragElement => {
                                    self.mouse_input_mode = MouseInputMode::DragElement((focused_element.clone(), *focused_window))
                                }
                                // TODO: should just move immediately ?
                                ClickAction::MoveItem(..) => {}
                                ClickAction::OpenWindow(prototype_window) => interface.open_window(focus_state, prototype_window.as_ref()),
                                ClickAction::CloseWindow => interface.close_window(focus_state, *focused_window),
                            }
                        }
                    }
                }
            }
        } else {
            if self.get_key(VirtualKeyCode::Tab).pressed() {
                interface.first_focused_element(focus_state);
            }

            if self.get_key(VirtualKeyCode::Escape).pressed() {
                events.push(UserEvent::OpenMenuWindow);
            }

            if self.get_key(VirtualKeyCode::I).pressed() {
                events.push(UserEvent::OpenInventoryWindow);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::M).pressed() {
                events.push(UserEvent::OpenMapsWindow);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::R).pressed() {
                events.push(UserEvent::OpenRenderSettingsWindow);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::T).pressed() {
                events.push(UserEvent::OpenTimeWindow);
            }

            #[cfg(feature = "debug_network")]
            if self.get_key(VirtualKeyCode::P).pressed() {
                events.push(UserEvent::OpenPacketWindow);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::LShift).pressed() && render_settings.use_debug_camera {
                events.push(UserEvent::CameraAccelerate);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::LShift).released() && render_settings.use_debug_camera {
                events.push(UserEvent::CameraDecelerate);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::F).pressed() {
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
            if self.get_key(VirtualKeyCode::W).down() && render_settings.use_debug_camera {
                events.push(UserEvent::CameraMoveForward);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::S).down() && render_settings.use_debug_camera {
                events.push(UserEvent::CameraMoveBackward);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::A).down() && render_settings.use_debug_camera {
                events.push(UserEvent::CameraMoveLeft);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::D).down() && render_settings.use_debug_camera {
                events.push(UserEvent::CameraMoveRight);
            }

            #[cfg(feature = "debug")]
            if self.get_key(VirtualKeyCode::Space).down() && render_settings.use_debug_camera {
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

        // TODO: this will fail if the user hovers over an entity that changes the
        // cursor and then immediately over a different one that doesn't,
        // because main wont set the default cursor
        if self.mouse_input_mode.is_none() && !matches!(mouse_target, Some(PickerTarget::Entity(_))) {
            interface.set_mouse_cursor_state(MouseCursorState::Default, client_tick);
        }

        if focus_state.did_hovered_element_change(&hovered_element) {
            if let Some(window_index) = focus_state.previous_hovered_window() {
                interface.schedule_rerender_window(window_index);
            }

            if let Some(window_index) = window_index {
                interface.schedule_rerender_window(window_index);
            }
        }

        if focus_state.did_focused_element_change() {
            if let Some(window_index) = focus_state.previous_focused_window() {
                interface.schedule_rerender_window(window_index);
            }

            if let Some(window_index) = focus_state.focused_window() {
                interface.schedule_rerender_window(window_index);
            }
        }

        let focused_element = focus_state.update(&hovered_element, window_index);

        (events, hovered_element, focused_element, mouse_target)
    }

    pub fn get_mouse_position(&self) -> Vector2<f32> {
        self.new_mouse_position
    }

    pub fn get_mouse_mode(&self) -> &MouseInputMode {
        &self.mouse_input_mode
    }
}
