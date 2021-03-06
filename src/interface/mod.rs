#[macro_use]
pub mod types;
pub mod traits;
pub mod elements;
pub mod windows;
pub mod cursor;

use derive_new::new;
use std::rc::Rc;
use std::cell::RefCell;
use vulkano::sync::GpuFuture;

use crate::graphics::{Renderer, InterfaceRenderer, DeferredRenderer, Color};
use crate::loaders::{ SpriteLoader, ActionLoader };
use crate::types::maths::Vector2;

pub use self::types::{ StateProvider, ClickAction, Size };
pub use self::windows::*;
pub use self::cursor::MouseCursor;

use self::types::*;
use self::traits::*;

#[derive(new)]
struct DialogHandle {
    elements: Rc<RefCell<Vec<DialogElement>>>,
    changed: Rc<RefCell<bool>>,
    clear: bool,
}

pub struct Interface {
    windows: Vec<(Box<dyn Window>, bool, bool)>,
    window_cache: WindowCache,
    interface_settings: InterfaceSettings,
    avalible_space: Size,
    theme: Theme,
    dialog_handle: Option<DialogHandle>,
    mouse_cursor: MouseCursor,
    reresolve: bool,
    rerender: bool,
}

impl Interface {

    pub fn new(sprite_loader: &mut SpriteLoader, action_loader: &mut ActionLoader, texture_future: &mut Box<dyn GpuFuture + 'static>, avalible_space: Size) -> Self {

        let window_cache = WindowCache::new();
        let interface_settings = InterfaceSettings::new();
        let theme = Theme::new(&interface_settings.theme_file);
        let dialog_handle = None;
        let mouse_cursor = MouseCursor::new(sprite_loader, action_loader, texture_future);

        Self {
            windows: Vec::new(),
            window_cache,
            interface_settings,
            avalible_space,
            theme,
            dialog_handle,
            mouse_cursor,
            reresolve: false,
            rerender: true, // set to true initially to clear the interface buffer
        }
    }

    pub fn reload_theme(&mut self) {
        if self.theme.reload(&self.interface_settings.theme_file) {
            self.reresolve = true;
        }
    }

    pub fn save_theme(&self) {
        self.theme.save(&self.interface_settings.theme_file);
    }

    pub fn schedule_rerender_window(&mut self, window_index: usize) {
        if window_index < self.windows.len() {
            let (window, _reresolve, rerender) = &mut self.windows[window_index];

            match window.has_transparency(&self.theme) {
                true => self.rerender = true,
                false => *rerender = true,
            }
        }
    }

    pub fn update(&mut self) -> (bool, bool) {

        for (window, _reresolve, rerender) in &mut self.windows {
            if let Some(change_event) = window.update() {
                match change_event {
                    ChangeEvent::Reresolve => self.reresolve = true,
                    ChangeEvent::Rerender => self.rerender = true,
                    ChangeEvent::RerenderWindow => {
                        match window.has_transparency(&self.theme) {
                            true => self.rerender = true,
                            false => *rerender = true,
                        }
                    },
                }
            }
        }

        for (window, reresolve, rerender) in &mut self.windows {

            if self.reresolve || *reresolve {

                let (_position, previous_size) = window.get_area();
                let (window_class, new_position, new_size) = window.resolve(&self.interface_settings, &self.theme, self.avalible_space);

                if let Some(window_class) = window_class {
                    self.window_cache.register_window(window_class, new_position, new_size);
                }

                self.rerender |= previous_size.x > new_size.x || previous_size.y > new_size.y;

                match window.has_transparency(&self.theme) {
                    true => self.rerender = true,
                    false => *rerender = true,
                }
                *reresolve = false;
            }
        }

        self.rerender |= self.reresolve;
        self.reresolve = false;

        (self.rerender, self.rerender | self.windows.iter().any(|(_window, _reresolve, rerender)| *rerender))
    }

    pub fn update_window_size(&mut self, screen_size: Size) {
        self.avalible_space = screen_size;
        self.reresolve = true;
    }

    pub fn hovered_element(&self, mouse_position: Position) -> (Option<ElementCell>, Option<usize>) {

        for (window_index, (window, _reresolve, _rerender)) in self.windows.iter().enumerate().rev() {
            match window.hovered_element(mouse_position) {
                HoverInformation::Element(hovered_element) => return (Some(hovered_element), Some(window_index)),
                HoverInformation::Hovered | HoverInformation::Ignored => return (None, Some(window_index)),
                HoverInformation::Missed=> {},
            }
        }

        (None, None)
    }

    pub fn move_window_to_top(&mut self, window_index: usize) -> usize {
        let (window, reresolve, _rerender) = self.windows.remove(window_index);
        let new_window_index = self.windows.len();
        let has_transparency = window.has_transparency(&self.theme);

        self.windows.push((window, reresolve, !has_transparency));
        self.rerender |= has_transparency;

        new_window_index
    }

    pub fn left_click_element(&mut self, hovered_element: &ElementCell, window_index: usize) -> Option<ClickAction> {
        let (_window, reresolve, _rerender) = &mut self.windows[window_index];
        hovered_element.borrow_mut().left_click(reresolve)
    }

    pub fn right_click_element(&mut self, hovered_element: &ElementCell, window_index: usize) -> Option<ClickAction> {
        let (_window, reresolve, _rerender) = &mut self.windows[window_index];
        hovered_element.borrow_mut().right_click(reresolve)
    }

    pub fn drag_element(&mut self, element: &ElementCell, _window_index: usize, mouse_delta: Position) {
        //let (_window, _reresolve, _rerender) = &mut self.windows[window_index];

        if let Some(change_event) = element.borrow_mut().drag(mouse_delta) {
            match change_event {
                ChangeEvent::Reresolve => self.reresolve = true,
                ChangeEvent::Rerender => self.rerender = true,
                ChangeEvent::RerenderWindow => panic!(),
            }
        }
    }

    pub fn move_window(&mut self, window_index: usize, offset: Position) {

        if let Some((window_class, position)) = self.windows[window_index].0.offset(self.avalible_space, offset) {
            self.window_cache.update_position(window_class, position);
        }

        self.rerender = true;
    }

    pub fn resize_window(&mut self, window_index: usize, growth: Size) {
        let (window, reresolve, _rerender) = &mut self.windows[window_index];

        let (_position, previous_size) = window.get_area();
        let (window_class, new_size) = window.resize(&self.interface_settings, &self.theme, self.avalible_space, growth);

        if previous_size != new_size {

            if let Some(window_class) = window_class {
                self.window_cache.update_size(window_class, new_size);
            }

            *reresolve = true;
            self.rerender |= previous_size.x > new_size.x || previous_size.y > new_size.y;
        }
    }

    fn flag_rerender_windows(&mut self, start_index: usize, area: Option<(Position, Size)>) {

        for window_index in start_index..self.windows.len() {

            let rerender = self.windows[window_index].2;
            let is_hovering = |(position, scale)| self.windows[window_index].0.hovers_area(position, scale);

            if rerender || area.map(is_hovering).unwrap_or(false) {

                let (position, scale) = {
                    let (window, _reresolve, rerender) = &mut self.windows[window_index];
                    *rerender = true;
                    window.get_area()
                };

                self.flag_rerender_windows(window_index + 1, Some((position, scale)));
            }
        }
    }

    pub fn render(&mut self, render_target: &mut <InterfaceRenderer as Renderer>::Target, renderer: &InterfaceRenderer, state_provider: &StateProvider, hovered_element: Option<ElementCell>) {

        let hovered_element = hovered_element.map(|element| unsafe { &*element.as_ptr() });

        if !self.rerender {
            self.flag_rerender_windows(0, None);
        }

        for (window, _reresolve, rerender) in &mut self.windows {
            if self.rerender || *rerender {
                window.render(render_target, renderer, state_provider, &self.interface_settings, &self.theme, hovered_element);
                *rerender = false;
            }
        }

        self.rerender = false;
    }

    pub fn render_hover_text(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, text: &str, mouse_position: Position) {
        let offset = Vector2::new(text.len() as f32 * -3.0, 20.0);
        renderer.render_text(render_target, text, mouse_position + offset + Vector2::new(1.0, 1.0), Color::monochrome(0), 12.0); // move variables into theme
        renderer.render_text(render_target, text, mouse_position + offset, Color::monochrome(255), 12.0); // move variables into theme
    }

    pub fn render_frames_per_second(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, frames_per_second: usize) {
        renderer.render_text(render_target, &frames_per_second.to_string(), *self.theme.overlay.text_offset * *self.interface_settings.scaling, *self.theme.overlay.foreground_color, *self.theme.overlay.font_size * *self.interface_settings.scaling);
    }

    pub fn render_mouse_cursor(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, mouse_position: Position) {
        self.mouse_cursor.render(render_target, renderer, mouse_position, *self.theme.cursor.color);
    }

    fn window_exists(&self, window_class: Option<&str>) -> bool {
        match window_class {
            Some(window_class) => self.windows.iter().any(|window| window.0.get_window_class().map_or(false, |other_window_class| window_class == other_window_class)),
            None => false,
        }
    }

    fn open_new_window(&mut self, window: Box<dyn Window + 'static>) {
        self.windows.push((window, true, true));
    }

    pub fn open_window(&mut self, prototype_window: &dyn PrototypeWindow) {
        if !self.window_exists(prototype_window.window_class()) {
            let window = prototype_window.to_window(&self.window_cache, &self.interface_settings, self.avalible_space);
            self.open_new_window(window);
        }
    }

    pub fn open_dialog_window(&mut self, text: String, npc_id: u32) {
        if let Some(dialog_handle) = &mut self.dialog_handle {
            let mut elements = dialog_handle.elements.borrow_mut();

            if dialog_handle.clear {
                elements.clear();
                dialog_handle.clear = false;
            }

            elements.push(DialogElement::Text(text));
            *dialog_handle.changed.borrow_mut() = true;
        } else {
            let (window, elements, changed) = DialogWindow::new(text, npc_id);
            self.dialog_handle = Some(DialogHandle::new(elements, changed, false));
            self.open_window(&window);
        }
    }

    pub fn add_next_button(&mut self) {
        if let Some(dialog_handle) = &mut self.dialog_handle {

            let mut elements = dialog_handle.elements.borrow_mut();
            elements.push(DialogElement::NextButton);

            *dialog_handle.changed.borrow_mut() = true;
            dialog_handle.clear = true;
        }
    }

    pub fn add_close_button(&mut self) {
        if let Some(dialog_handle) = &self.dialog_handle {

            let mut elements = dialog_handle.elements.borrow_mut();
            elements.retain(|element| *element != DialogElement::NextButton);
            elements.push(DialogElement::CloseButton);

            *dialog_handle.changed.borrow_mut() = true;
        }
    }

    pub fn add_choice_buttons(&mut self, choices: Vec<String>) {
        if let Some(dialog_handle) = &self.dialog_handle {

            let mut elements = dialog_handle.elements.borrow_mut();
            elements.retain(|element| *element != DialogElement::NextButton);

            choices
                .into_iter()
                .enumerate()
                .for_each(|(index, choice)| elements.push(DialogElement::ChoiceButton(choice, index as i8 + 1)));

            elements.push(DialogElement::ChoiceButton("cancel".to_string(), -1));

            *dialog_handle.changed.borrow_mut() = true;
        }
    }

    pub fn close_dialog_window(&mut self) {
        self.close_window_with_class(DialogWindow::WINDOW_CLASS);
        self.dialog_handle = None;
    }

    pub fn handle_result<T>(&mut self, result: Result<T, String>) {
        if let Err(message) = result {
            self.open_window(&ErrorWindow::new(message));
        } 
    }

    #[cfg(feature = "debug")]
    pub fn open_theme_viewer_window(&mut self) {
        if !self.window_exists(self.theme.window_class()) {
            let window = self.theme.to_window(&self.window_cache, &self.interface_settings, self.avalible_space);
            self.open_new_window(window);
        }
    }

    pub fn close_window(&mut self, window_index: usize) {
        self.windows.remove(window_index);
        self.rerender = true;
    }

    pub fn close_window_with_class(&mut self, window_class: &str) {
        self.windows.retain(|window| !window.0.get_window_class().map_or(false, |other_window_class| window_class == other_window_class));
        self.rerender = true;
    }
}
