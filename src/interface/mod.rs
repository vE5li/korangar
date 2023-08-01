mod event;
mod layout;
mod provider;
mod settings;
mod state;
mod theme;
#[macro_use]
mod elements;
mod cursor;
mod windows;

use std::cell::RefCell;
use std::rc::Rc;

use cgmath::Vector2;
use derive_new::new;
use option_ext::OptionExt;
use procedural::profile;

pub use self::cursor::*;
pub use self::elements::*;
pub use self::event::*;
pub use self::layout::*;
pub use self::provider::StateProvider;
pub use self::settings::InterfaceSettings;
pub use self::state::{Remote, TrackedState};
pub use self::theme::Theme;
pub use self::windows::*;
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{Color, DeferredRenderer, InterfaceRenderer, Renderer};
use crate::input::{FocusState, Grabbed, MouseInputMode, UserEvent};
use crate::loaders::{ActionLoader, FontLoader, GameFileLoader, SpriteLoader};
use crate::network::{ClientTick, EntityId};

// TODO: move this
pub type Selector = Box<dyn Fn() -> bool>;
pub type ColorSelector = Box<dyn Fn(&Theme) -> Color>;
pub type FontSizeSelector = Box<dyn Fn(&Theme) -> f32>;

pub trait ElementEvent {
    fn trigger(&mut self) -> Option<ClickAction>;
}

impl<F> ElementEvent for Box<F>
where
    F: FnMut() -> Option<ClickAction> + 'static,
{
    fn trigger(&mut self) -> Option<ClickAction> {
        self()
    }
}

impl ElementEvent for UserEvent {
    fn trigger(&mut self) -> Option<ClickAction> {
        Some(ClickAction::Event(self.clone()))
    }
}

#[derive(new)]
struct DialogHandle {
    elements: TrackedState<Vec<DialogElement>>,
    clear: bool,
}

pub struct Interface {
    windows: Vec<(Window, bool, bool)>,
    window_cache: WindowCache,
    interface_settings: InterfaceSettings,
    available_space: Size,
    theme: Theme,
    dialog_handle: Option<DialogHandle>,
    mouse_cursor: MouseCursor,
    mouse_cursor_hidden: bool,
    reresolve: bool,
    rerender: bool,
}

impl Interface {
    pub fn new(
        game_file_loader: &mut GameFileLoader,
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        available_space: Size,
    ) -> Self {
        let window_cache = WindowCache::new();
        let interface_settings = InterfaceSettings::new();
        let theme = Theme::new(&interface_settings.theme_file);
        let dialog_handle = None;
        let mouse_cursor = MouseCursor::new(game_file_loader, sprite_loader, action_loader);
        let mouse_cursor_hidden = false;

        Self {
            windows: Vec::new(),
            window_cache,
            interface_settings,
            available_space,
            theme,
            dialog_handle,
            mouse_cursor,
            mouse_cursor_hidden,
            reresolve: false,
            rerender: true, // set to true initially to clear the interface buffer
        }
    }

    pub fn set_theme_file(&mut self, theme_file: String) {
        self.interface_settings.theme_file = theme_file;
    }

    #[profile]
    pub fn save_theme(&self) {
        self.theme.save(&self.interface_settings.theme_file);
    }

    #[profile]
    pub fn reload_theme(&mut self) {
        if self.theme.reload(&self.interface_settings.theme_file) {
            self.reresolve = true;
        }
    }

    pub fn get_theme(&self) -> &Theme {
        &self.theme
    }

    pub fn schedule_rerender(&mut self) {
        self.rerender = true;
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

    // TODO: this is just a workaround until i find a better solution to make the
    // cursor always look correct.
    pub fn set_start_time(&mut self, client_tick: ClientTick) {
        self.mouse_cursor.set_start_time(client_tick);
    }

    #[profile("update user interface")]
    pub fn update(&mut self, font_loader: Rc<RefCell<FontLoader>>, focus_state: &mut FocusState, client_tick: ClientTick) -> (bool, bool) {
        self.mouse_cursor.update(client_tick);

        for (window, _reresolve, rerender) in &mut self.windows {
            #[cfg(feature = "debug")]

            profile_block!("update window");

            if let Some(change_event) = window.update() {
                match change_event {
                    ChangeEvent::Reresolve => self.reresolve = true,
                    ChangeEvent::Rerender => self.rerender = true,
                    ChangeEvent::RerenderWindow => match window.has_transparency(&self.theme) {
                        true => self.rerender = true,
                        false => *rerender = true,
                    },
                }
            }
        }

        let mut restore_focus = false;

        for (window_index, (window, reresolve, rerender)) in self.windows.iter_mut().enumerate() {
            if self.reresolve || *reresolve {
                #[cfg(feature = "debug")]

                profile_block!("reresolve window");

                let (_position, previous_size) = window.get_area();
                let (window_class, new_position, new_size) =
                    window.resolve(font_loader.clone(), &self.interface_settings, &self.theme, self.available_space);

                // should only ever be the last window
                if let Some(focused_index) = focus_state.focused_window() && focused_index == window_index {
                    restore_focus = true;
                }

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

        if restore_focus {
            self.restore_focus(focus_state);
        }

        self.rerender |= self.reresolve;
        self.reresolve = false;

        if !self.rerender {
            // We profile this block rather than the flag function itself because it calls
            // itself recursively
            #[cfg(feature = "debug")]
            profile_block!("flag rerender windows");

            self.flag_rerender_windows(0, None);
        }

        (
            self.rerender,
            self.rerender | self.windows.iter().any(|(_window, _reresolve, rerender)| *rerender),
        )
    }

    pub fn update_window_size(&mut self, screen_size: Size) {
        self.available_space = screen_size;
        self.reresolve = true;
    }

    #[profile("get hovered element")]
    pub fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> (Option<ElementCell>, Option<usize>) {
        for (window_index, (window, _reresolve, _rerender)) in self.windows.iter().enumerate().rev() {
            match window.hovered_element(mouse_position, mouse_mode) {
                HoverInformation::Element(hovered_element) => return (Some(hovered_element), Some(window_index)),
                HoverInformation::Hovered => return (None, Some(window_index)),
                HoverInformation::Missed => {}
            }
        }

        (None, None)
    }

    #[profile]
    pub fn move_window_to_top(&mut self, window_index: usize) -> usize {
        let (window, reresolve, _rerender) = self.windows.remove(window_index);
        let new_window_index = self.windows.len();
        let has_transparency = window.has_transparency(&self.theme);

        self.windows.push((window, reresolve, !has_transparency));
        self.rerender |= has_transparency;

        new_window_index
    }

    #[profile]
    pub fn left_click_element(&mut self, hovered_element: &ElementCell, window_index: usize) -> Option<ClickAction> {
        let (_window, reresolve, _rerender) = &mut self.windows[window_index];
        hovered_element.borrow_mut().left_click(reresolve) // TODO: add same change_event check as
        // for input character ?
    }

    #[profile]
    pub fn right_click_element(&mut self, hovered_element: &ElementCell, window_index: usize) -> Option<ClickAction> {
        let (_window, reresolve, _rerender) = &mut self.windows[window_index];
        hovered_element.borrow_mut().right_click(reresolve) // TODO: add same change_event check as
        // for input character ?
    }

    #[profile]
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

    #[profile]
    pub fn scroll_element(&mut self, element: &ElementCell, window_index: usize, scroll_delta: f32) {
        let (_, _, rerender) = &mut self.windows[window_index];

        if let Some(change_event) = element.borrow_mut().scroll(scroll_delta) {
            match change_event {
                ChangeEvent::Reresolve => self.reresolve = true,
                ChangeEvent::Rerender => self.rerender = true,
                ChangeEvent::RerenderWindow => *rerender = true,
            }
        }
    }

    #[profile]
    pub fn input_character_element(&mut self, element: &ElementCell, window_index: usize, character: char) -> Option<ClickAction> {
        let (window, _reresolve, rerender) = &mut self.windows[window_index];
        let has_transparency = window.has_transparency(&self.theme);

        if let Some(click_event) = element.borrow_mut().input_character(character) {
            match click_event {
                ClickAction::ChangeEvent(change_event) => match change_event {
                    ChangeEvent::Reresolve => self.reresolve = true,
                    ChangeEvent::Rerender => self.rerender = true,
                    ChangeEvent::RerenderWindow => match has_transparency {
                        true => self.rerender = true,
                        false => *rerender = true,
                    },
                },
                other => return Some(other),
            }
        }

        None
    }

    #[profile]
    pub fn move_window(&mut self, window_index: usize, offset: Position) {
        if let Some((window_class, position)) = self.windows[window_index].0.offset(self.available_space, offset) {
            self.window_cache.update_position(window_class, position);
        }

        self.rerender = true;
    }

    #[profile]
    pub fn resize_window(&mut self, window_index: usize, growth: Size) {
        let (window, reresolve, _rerender) = &mut self.windows[window_index];

        let (_position, previous_size) = window.get_area();
        let (window_class, new_size) = window.resize(&self.interface_settings, &self.theme, self.available_space, growth);

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

                    if window.has_transparency(&self.theme) {
                        self.rerender = true;
                        return;
                    }

                    *rerender = true;
                    window.get_area()
                };

                self.flag_rerender_windows(window_index + 1, Some((position, scale)));
            }
        }
    }

    #[profile("render user interface")]
    pub fn render(
        &mut self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state_provider: &StateProvider,
        hovered_element: Option<ElementCell>,
        focused_element: Option<ElementCell>,
        mouse_mode: &MouseInputMode,
    ) {
        let hovered_element = hovered_element.map(|element| unsafe { &*element.as_ptr() });
        let focused_element = focused_element.map(|element| unsafe { &*element.as_ptr() });

        for (window, _reresolve, rerender) in &mut self.windows {
            if self.rerender || *rerender {
                #[cfg(feature = "debug")]
                profile_block!("render window");

                window.render(
                    render_target,
                    renderer,
                    state_provider,
                    &self.interface_settings,
                    &self.theme,
                    hovered_element,
                    focused_element,
                    mouse_mode,
                );
                *rerender = false;
            }
        }

        self.rerender = false;
    }

    #[profile]
    pub fn render_hover_text(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        text: &str,
        mouse_position: Position,
    ) {
        let offset = Vector2::new(text.len() as f32 * -3.0, 20.0);
        renderer.render_text(
            render_target,
            text,
            mouse_position + offset + Vector2::new(1.0, 1.0),
            Color::monochrome(0),
            12.0,
        ); // move variables into theme
        renderer.render_text(render_target, text, mouse_position + offset, Color::monochrome(255), 12.0); // move variables into theme
    }

    #[profile]
    #[cfg(feature = "debug")]
    pub fn render_frames_per_second(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        frames_per_second: usize,
    ) {
        renderer.render_text(
            render_target,
            &frames_per_second.to_string(),
            *self.theme.overlay.text_offset * *self.interface_settings.scaling,
            *self.theme.overlay.foreground_color,
            *self.theme.overlay.font_size * *self.interface_settings.scaling,
        );
    }

    #[profile]
    pub fn render_mouse_cursor(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        mouse_position: Position,
        grabbed: Option<Grabbed>,
    ) {
        if !self.mouse_cursor_hidden {
            #[cfg(feature = "debug")]
            profile_block!("render mouse cursor");

            self.mouse_cursor.render(
                render_target,
                renderer,
                mouse_position,
                grabbed,
                *self.theme.cursor.color,
                &self.interface_settings,
            );
        }
    }

    #[profile("check window exists")]
    fn window_exists(&self, window_class: Option<&str>) -> bool {
        match window_class {
            Some(window_class) => self.windows.iter().any(|window| {
                window
                    .0
                    .get_window_class()
                    .map_or(false, |other_window_class| window_class == other_window_class)
            }),
            None => false,
        }
    }

    fn open_new_window(&mut self, focus_state: &mut FocusState, window: Window) {
        self.windows.push((window, true, true));
        focus_state.set_focused_window(self.windows.len() - 1);
    }

    #[profile]
    pub fn open_window(&mut self, focus_state: &mut FocusState, prototype_window: &dyn PrototypeWindow) {
        if !self.window_exists(prototype_window.window_class()) {
            let window = prototype_window.to_window(&self.window_cache, &self.interface_settings, self.available_space);
            self.open_new_window(focus_state, window);
        }
    }

    #[profile]
    pub fn open_dialog_window(&mut self, focus_state: &mut FocusState, text: String, npc_id: EntityId) {
        if let Some(dialog_handle) = &mut self.dialog_handle {
            dialog_handle.elements.with_mut(|elements, changed| {
                if dialog_handle.clear {
                    elements.clear();
                    dialog_handle.clear = false;
                }

                elements.push(DialogElement::Text(text));
                changed();
            });
        } else {
            let (window, elements) = DialogWindow::new(text, npc_id);
            self.dialog_handle = Some(DialogHandle::new(elements, false));
            self.open_window(focus_state, &window);
        }
    }

    #[profile]
    pub fn add_next_button(&mut self) {
        if let Some(dialog_handle) = &mut self.dialog_handle {
            dialog_handle.elements.push(DialogElement::NextButton);
            dialog_handle.clear = true;
        }
    }

    #[profile]
    pub fn add_close_button(&mut self) {
        if let Some(dialog_handle) = &mut self.dialog_handle {
            dialog_handle.elements.with_mut(|elements, changed| {
                elements.retain(|element| *element != DialogElement::NextButton);
                elements.push(DialogElement::CloseButton);
                changed();
            });
        }
    }

    #[profile]
    pub fn add_choice_buttons(&mut self, choices: Vec<String>) {
        if let Some(dialog_handle) = &mut self.dialog_handle {
            dialog_handle.elements.with_mut(move |elements, changed| {
                elements.retain(|element| *element != DialogElement::NextButton);

                choices
                    .into_iter()
                    .enumerate()
                    .for_each(|(index, choice)| elements.push(DialogElement::ChoiceButton(choice, index as i8 + 1)));

                changed();
            });
        }
    }

    pub fn handle_result<T>(&mut self, focus_state: &mut FocusState, result: Result<T, String>) {
        if let Err(message) = result {
            self.open_window(focus_state, &ErrorWindow::new(message));
        }
    }

    #[profile]
    #[cfg(feature = "debug")]
    pub fn open_theme_viewer_window(&mut self, focus_state: &mut FocusState) {
        if !self.window_exists(self.theme.window_class()) {
            let window = self
                .theme
                .to_window(&self.window_cache, &self.interface_settings, self.available_space);

            self.open_new_window(focus_state, window);
        }
    }

    #[profile]
    pub fn close_window(&mut self, focus_state: &mut FocusState, window_index: usize) {
        let (window, ..) = self.windows.remove(window_index);
        self.rerender = true;

        // drop window in another thread to avoid frame drops when deallocation a large
        // amount of elements
        std::thread::spawn(move || drop(window));

        // TODO: only if tab mode
        self.restore_focus(focus_state);
    }

    pub fn get_window(&self, window_index: usize) -> &Window {
        &self.windows[window_index].0
    }

    #[profile]
    pub fn close_window_with_class(&mut self, focus_state: &mut FocusState, window_class: &str) {
        let index_from_back = self
            .windows
            .iter()
            .rev()
            .map(|(window, ..)| window.get_window_class())
            .position(|class_option| class_option.contains(&window_class))
            .unwrap();
        let index = self.windows.len() - 1 - index_from_back;

        self.close_window(focus_state, index);
    }

    #[profile]
    pub fn close_dialog_window(&mut self, focus_state: &mut FocusState) {
        self.close_window_with_class(focus_state, DialogWindow::WINDOW_CLASS);
        self.dialog_handle = None;
    }

    #[profile]
    pub fn close_all_windows(&mut self, focus_state: &mut FocusState) {
        for index in (0..self.windows.len()).rev() {
            self.close_window(focus_state, index);
        }
    }

    #[profile]
    pub fn set_mouse_cursor_state(&mut self, state: MouseCursorState, client_tick: ClientTick) {
        self.mouse_cursor.set_state(state, client_tick)
    }

    #[profile("get first focused element")]
    pub fn first_focused_element(&self, focus_state: &mut FocusState) {
        if self.windows.is_empty() {
            return;
        }

        let window_index = self.windows.len() - 1;
        let element = self.windows.last().unwrap().0.first_focused_element();

        focus_state.set_focused_element(element, window_index);
    }

    #[profile]
    pub fn restore_focus(&self, focus_state: &mut FocusState) {
        if self.windows.is_empty() {
            return;
        }

        let window_index = self.windows.len() - 1;
        let element = self.windows.last().unwrap().0.restore_focus();

        focus_state.set_focused_element(element, window_index);
    }

    pub fn hide_mouse_cursor(&mut self) {
        self.mouse_cursor_hidden = true;
    }

    pub fn show_mouse_cursor(&mut self) {
        self.mouse_cursor_hidden = false;
    }
}
