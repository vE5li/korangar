#[macro_use]
pub mod types;
pub mod traits;
pub mod elements;
pub mod windows;

use crate::graphics::Renderer;

pub use self::types::{ StateProvider, ClickAction, Size };
pub use self::windows::*;

use self::types::*;
use self::traits::*;

pub struct Interface {
    windows: Vec<(Box<dyn Window>, bool, bool)>,
    window_cache: WindowCache,
    interface_settings: InterfaceSettings,
    avalible_space: Size,
    theme: Theme,
    reresolve: bool,
    rerender: bool,
}

impl Interface {

    pub fn new(avalible_space: Size) -> Self {

        let window_cache = WindowCache::new();
        let interface_settings = InterfaceSettings::new();
        let theme = Theme::new(&interface_settings.theme_file);

        Self {
            windows: Vec::new(),
            window_cache,
            interface_settings,
            avalible_space,
            theme,
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

    pub fn schedule_rerender(&mut self) {
        self.rerender = true;
    }

    pub fn schedule_rerender_window(&mut self, window_index: usize) {
        if window_index < self.windows.len() {
            self.windows[window_index].2 = true;
        }
    }

    pub fn update(&mut self) -> bool {

        for (window, _reresolve, rerender) in &mut self.windows {
            if let Some(change_event) = window.update() {
                match change_event {
                    ChangeEvent::Reresolve => self.reresolve = true,
                    ChangeEvent::Rerender => self.rerender = true,
                    ChangeEvent::RerenderWindow => *rerender = true,
                }
            }
        }

        for (window, reresolve, rerender) in &mut self.windows {

            if self.reresolve || *reresolve {

                let (_position, previous_size) = window.get_area();
                let (window_class, new_position, new_size) = window.resolve(&self.interface_settings, &self.theme, self.avalible_space);

                if previous_size != new_size {

                    if let Some(window_class) = window_class {
                        self.window_cache.register_window(window_class, new_position, new_size);
                    }

                    self.rerender |= previous_size.x > new_size.x || previous_size.y > new_size.y;
                }

                *rerender |= *reresolve;
                *reresolve = false;
            }
        }

        self.rerender |= self.reresolve;
        self.reresolve = false;

        self.rerender
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

        self.windows.push((window, reresolve, true));

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

    pub fn render(&mut self, renderer: &mut Renderer, state_provider: &StateProvider, hovered_element: Option<ElementCell>) {

        let hovered_element = hovered_element.map(|element| unsafe { &*element.as_ptr() });

        if !self.rerender {
            self.flag_rerender_windows(0, None);
        }

        for (window, _reresolve, rerender) in &mut self.windows {
            if self.rerender || *rerender {
                window.render(renderer, state_provider, &self.interface_settings, &self.theme, hovered_element);
                *rerender = false;
            }
        }

        self.rerender = false;
    }

    pub fn render_frames_per_second(&self, renderer: &mut Renderer, frames_per_second: usize) {
        renderer.render_dynamic_text(&frames_per_second.to_string(), *self.theme.overlay.text_offset * *self.interface_settings.scaling, *self.theme.overlay.foreground_color, *self.theme.overlay.font_size * *self.interface_settings.scaling);
    }

    fn window_exists(&self, window_class: Option<&str>) -> bool {
        match window_class {
            Some(window_class) => self.windows.iter().any(|window| window.0.window_class_matches(window_class)),
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
        self.windows.retain(|window| !window.0.window_class_matches(window_class));
        self.rerender = true;
    }
}
