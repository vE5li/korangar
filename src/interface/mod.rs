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
use std::marker::{ConstParamTy, PhantomData};
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
pub use self::theme::{GameTheme, InterfaceTheme};
use self::theme::{Main, Menu, ThemeSelector, Themes};
pub use self::windows::*;
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{Color, DeferredRenderer, InterfaceRenderer, Renderer};
use crate::input::{FocusState, Grabbed, MouseInputMode, UserEvent};
use crate::loaders::{ActionLoader, FontLoader, GameFileLoader, SpriteLoader};
use crate::network::{ClientTick, EntityId};

// TODO: move this
pub type Selector = Box<dyn Fn() -> bool>;
pub type ColorSelector = Box<dyn Fn(&InterfaceTheme) -> Color>;
pub type FontSizeSelector = Box<dyn Fn(&InterfaceTheme) -> f32>;

pub trait ElementEvent {
    fn trigger(&mut self) -> Vec<ClickAction>;
}

impl<F> ElementEvent for Box<F>
where
    F: FnMut() -> Vec<ClickAction> + 'static,
{
    fn trigger(&mut self) -> Vec<ClickAction> {
        self()
    }
}

impl ElementEvent for UserEvent {
    fn trigger(&mut self) -> Vec<ClickAction> {
        vec![ClickAction::Event(self.clone())]
    }
}

#[derive(new)]
struct DialogHandle {
    elements: TrackedState<Vec<DialogElement>>,
    clear: bool,
}

#[derive(Clone)]
struct PerWindow;

#[derive(Clone)]
struct PostUpdate<T> {
    resolve: bool,
    render: bool,
    marker: PhantomData<T>,
}

impl<T> PostUpdate<T> {
    pub fn new() -> Self {
        Self {
            resolve: false,
            render: false,
            marker: PhantomData,
        }
    }

    pub fn render(&mut self) {
        self.render = true;
    }

    pub fn resolve(&mut self) {
        self.resolve = true;
    }

    pub fn with_render(mut self) -> Self {
        self.render = true;
        self
    }

    pub fn with_resolve(mut self) -> Self {
        self.resolve = true;
        self
    }

    pub fn needs_render(&self) -> bool {
        self.render
    }

    pub fn needs_resolve(&self) -> bool {
        self.resolve
    }

    pub fn take_render(&mut self) -> bool {
        let state = self.render;
        self.render = false;
        state
    }

    pub fn take_resolve(&mut self) -> bool {
        let state = self.resolve;
        self.resolve = false;
        state
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeKind {
    Menu,
    #[default]
    Main,
    Game,
}

impl ConstParamTy for ThemeKind {}

pub type Tracker<T> = Box<dyn Fn() -> Option<T>>;

pub struct Interface {
    windows: Vec<(Window, PostUpdate<PerWindow>)>,
    window_cache: WindowCache,
    interface_settings: InterfaceSettings,
    available_space: Size,
    themes: Themes,
    dialog_handle: Option<DialogHandle>,
    mouse_cursor: MouseCursor,
    mouse_cursor_hidden: bool,
    post_update: PostUpdate<Self>,
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
        let themes = Themes {
            theme_selector: ThemeSelector,
            menu: InterfaceTheme::new::<Menu>(interface_settings.menu_theme.get_file()),
            main: InterfaceTheme::new::<Main>(interface_settings.main_theme.get_file()),
            game: GameTheme::new(interface_settings.game_theme.get_file()),
        };
        let dialog_handle = None;
        let mouse_cursor = MouseCursor::new(game_file_loader, sprite_loader, action_loader);
        let mouse_cursor_hidden = false;
        // NOTE: We need to initially clear the interface buffer
        let post_update = PostUpdate::new().with_render();

        Self {
            windows: Vec::new(),
            window_cache,
            interface_settings,
            available_space,
            themes,
            dialog_handle,
            mouse_cursor,
            mouse_cursor_hidden,
            post_update,
        }
    }

    pub fn set_theme_file(&mut self, theme_file: String, theme_kind: ThemeKind) {
        match theme_kind {
            ThemeKind::Menu => self.interface_settings.menu_theme.set_file(theme_file),
            ThemeKind::Main => self.interface_settings.main_theme.set_file(theme_file),
            ThemeKind::Game => self.interface_settings.game_theme.set_file(theme_file),
        }
    }

    pub fn get_game_theme(&self) -> &GameTheme {
        &self.themes.game
    }

    #[profile]
    pub fn save_theme(&self, kind: ThemeKind) {
        match kind {
            ThemeKind::Menu => self.themes.menu.save(self.interface_settings.menu_theme.get_file()),
            ThemeKind::Main => self.themes.main.save(self.interface_settings.main_theme.get_file()),
            ThemeKind::Game => self.themes.game.save(self.interface_settings.game_theme.get_file()),
        }
    }

    #[profile]
    pub fn reload_theme(&mut self, kind: ThemeKind) {
        let success = match kind {
            ThemeKind::Menu => self.themes.menu.reload(self.interface_settings.menu_theme.get_file()),
            ThemeKind::Main => self.themes.main.reload(self.interface_settings.main_theme.get_file()),
            ThemeKind::Game => self.themes.game.reload(self.interface_settings.game_theme.get_file()),
        };

        if success {
            self.post_update.resolve();
        }
    }

    pub fn schedule_render(&mut self) {
        self.post_update.render();
    }

    pub fn schedule_render_window(&mut self, window_index: usize) {
        if window_index < self.windows.len() {
            let (_, post_update) = &mut self.windows[window_index];
            post_update.render();
        }
    }

    // TODO: this is just a workaround until i find a better solution to make the
    // cursor always look correct.
    pub fn set_start_time(&mut self, client_tick: ClientTick) {
        self.mouse_cursor.set_start_time(client_tick);
    }

    /// The update and render functions take care of merging the window specific
    /// flags with the interface wide flags.
    fn handle_change_event(post_update: &mut PostUpdate<Self>, window_post_update: &mut PostUpdate<PerWindow>, change_event: ChangeEvent) {
        if change_event.contains(ChangeEvent::RENDER_WINDOW) {
            window_post_update.render();
        }

        if change_event.contains(ChangeEvent::RESOLVE_WINDOW) {
            window_post_update.resolve();
        }

        if change_event.contains(ChangeEvent::RENDER) {
            post_update.render();
        }

        if change_event.contains(ChangeEvent::RESOLVE) {
            post_update.resolve();
        }
    }

    #[profile("update user interface")]
    pub fn update(&mut self, font_loader: Rc<RefCell<FontLoader>>, focus_state: &mut FocusState, client_tick: ClientTick) -> (bool, bool) {
        self.mouse_cursor.update(client_tick);

        for (window, post_update) in &mut self.windows {
            #[cfg(feature = "debug")]

            profile_block!("update window");

            if let Some(change_event) = window.update() {
                Self::handle_change_event(&mut self.post_update, post_update, change_event);
            }
        }

        let mut restore_focus = false;

        for (window_index, (window, post_update)) in self.windows.iter_mut().enumerate() {
            if self.post_update.needs_resolve() || post_update.take_resolve() {
                #[cfg(feature = "debug")]

                profile_block!("resolve window");

                let (_position, previous_size) = window.get_area();
                let theme = match window.get_theme_kind() {
                    ThemeKind::Menu => &self.themes.menu,
                    ThemeKind::Main => &self.themes.main,
                    _ => panic!(),
                };

                let (window_class, new_position, new_size) =
                    window.resolve(font_loader.clone(), &self.interface_settings, theme, self.available_space);

                // should only ever be the last window
                if let Some(focused_index) = focus_state.focused_window()
                    && focused_index == window_index
                {
                    restore_focus = true;
                }

                if let Some(window_class) = window_class {
                    self.window_cache.register_window(window_class, new_position, new_size);
                }

                // NOTE: If the window got smaller, we need to re-render the entire interface.
                // If it got bigger, we can just draw over the previous frame.
                match previous_size.x > new_size.x || previous_size.y > new_size.y {
                    true => self.post_update.render(),
                    false => post_update.render(),
                }
            }
        }

        if restore_focus {
            self.restore_focus(focus_state);
        }

        if self.post_update.take_resolve() {
            self.post_update.render();
        }

        if !self.post_update.needs_render() {
            // We profile this block rather than the flag function itself because it calls
            // itself recursively
            #[cfg(feature = "debug")]
            profile_block!("flag render windows");

            self.flag_render_windows(0, None);
        }

        let render_interface = self.post_update.needs_render();
        let render_window = self.post_update.needs_render() | self.windows.iter().any(|(_window, post_update)| post_update.needs_render());

        (render_interface, render_window)
    }

    pub fn update_window_size(&mut self, screen_size: Size) {
        self.available_space = screen_size;
        self.post_update.resolve();
    }

    #[profile("get hovered element")]
    pub fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> (Option<ElementCell>, Option<usize>) {
        for (window_index, (window, _)) in self.windows.iter().enumerate().rev() {
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
        let (window, post_update) = self.windows.remove(window_index);
        let new_window_index = self.windows.len();

        self.windows.push((window, post_update.with_render()));

        new_window_index
    }

    #[profile]
    pub fn left_click_element(&mut self, hovered_element: &ElementCell, window_index: usize) -> Vec<ClickAction> {
        let (_, post_update) = &mut self.windows[window_index];
        let mut resolve = false;

        let action = hovered_element.borrow_mut().left_click(&mut resolve); // TODO: add same change_event check as for input character ?

        if resolve {
            post_update.resolve();
        }

        action
    }

    #[profile]
    pub fn right_click_element(&mut self, hovered_element: &ElementCell, window_index: usize) -> Vec<ClickAction> {
        let (_, post_update) = &mut self.windows[window_index];
        let mut resolve = false;

        let action = hovered_element.borrow_mut().right_click(&mut resolve); // TODO: add same change_event check as for input character ?

        if resolve {
            post_update.resolve();
        }

        action
    }

    #[profile]
    pub fn drag_element(&mut self, element: &ElementCell, _window_index: usize, mouse_delta: Position) {
        //let (_window, post_update) = &mut self.windows[window_index];

        if let Some(change_event) = element.borrow_mut().drag(mouse_delta) {
            // TODO: Use the window post_update here (?)
            Self::handle_change_event(&mut self.post_update, &mut PostUpdate::new(), change_event);
        }
    }

    #[profile]
    pub fn scroll_element(&mut self, element: &ElementCell, window_index: usize, scroll_delta: f32) {
        let (_, post_update) = &mut self.windows[window_index];

        if let Some(change_event) = element.borrow_mut().scroll(scroll_delta) {
            Self::handle_change_event(&mut self.post_update, post_update, change_event);
        }
    }

    #[profile]
    pub fn input_character_element(&mut self, element: &ElementCell, window_index: usize, character: char) -> Vec<ClickAction> {
        let (_, post_update) = &mut self.windows[window_index];
        let mut propagated_actions = Vec::new();

        for action in element.borrow_mut().input_character(character) {
            match action {
                ClickAction::ChangeEvent(change_event) => Self::handle_change_event(&mut self.post_update, post_update, change_event),
                other => propagated_actions.push(other),
            }
        }

        propagated_actions
    }

    #[profile]
    pub fn move_window(&mut self, window_index: usize, offset: Position) {
        if let Some((window_class, position)) = self.windows[window_index].0.offset(self.available_space, offset) {
            self.window_cache.update_position(window_class, position);
        }

        self.post_update.render();
    }

    #[profile]
    pub fn resize_window(&mut self, window_index: usize, growth: Size) {
        let (window, post_update) = &mut self.windows[window_index];

        let theme = match window.get_theme_kind() {
            ThemeKind::Menu => &self.themes.menu,
            ThemeKind::Main => &self.themes.main,
            _ => panic!(),
        };
        let (_position, previous_size) = window.get_area();

        let (window_class, new_size) = window.resize(&self.interface_settings, theme, self.available_space, growth);

        if previous_size != new_size {
            if let Some(window_class) = window_class {
                self.window_cache.update_size(window_class, new_size);
            }

            post_update.resolve();

            if previous_size.x > new_size.x || previous_size.y > new_size.y {
                self.post_update.render();
            }
        }
    }

    /// This function is solely responsible for making sure that trying to
    /// re-render a window with transparency will result in re-rendering the
    /// entire interface. This serves as a single point of truth and simplifies
    /// the rest of the code.
    fn flag_render_windows(&mut self, start_index: usize, area: Option<(Position, Size)>) {
        for window_index in start_index..self.windows.len() {
            let needs_render = self.windows[window_index].1.needs_render();
            let is_hovering = |(position, scale)| self.windows[window_index].0.hovers_area(position, scale);

            if needs_render || area.map(is_hovering).unwrap_or(false) {
                let (position, scale) = {
                    let (window, post_update) = &mut self.windows[window_index];

                    if window.has_transparency(&self.themes.main) {
                        self.post_update.render();
                        return;
                    }

                    post_update.render();
                    window.get_area()
                };

                self.flag_render_windows(window_index + 1, Some((position, scale)));
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

        for (window, post_update) in &mut self.windows {
            if post_update.take_render() || self.post_update.needs_render() {
                #[cfg(feature = "debug")]
                profile_block!("render window");

                let theme = match window.get_theme_kind() {
                    ThemeKind::Menu => &self.themes.menu,
                    ThemeKind::Main => &self.themes.main,
                    _ => panic!(),
                };

                window.render(
                    render_target,
                    renderer,
                    state_provider,
                    &self.interface_settings,
                    theme,
                    hovered_element,
                    focused_element,
                    mouse_mode,
                );
            }
        }

        self.post_update.take_render();
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
            *self.themes.game.overlay.text_offset * *self.interface_settings.scaling,
            *self.themes.game.overlay.foreground_color,
            *self.themes.game.overlay.font_size * *self.interface_settings.scaling,
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
                *self.themes.game.cursor.color,
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
        self.windows.push((window, PostUpdate::new().with_resolve()));
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
    pub fn open_popup(
        &mut self,
        element: ElementCell,
        position_tracker: Tracker<Position>,
        size_tracker: Tracker<Size>,
        window_index: usize,
    ) {
        let entry = &mut self.windows[window_index];
        entry.0.open_popup(element, position_tracker, size_tracker);
        entry.1.resolve();
    }

    #[profile]
    pub fn close_popup(&mut self, window_index: usize) {
        let entry = &mut self.windows[window_index];
        entry.0.close_popup();
        entry.1.render();
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
        if !self.window_exists(self.themes.window_class()) {
            let window = self
                .themes
                .to_window(&self.window_cache, &self.interface_settings, self.available_space);

            self.open_new_window(focus_state, window);
        }
    }

    #[profile]
    pub fn close_window(&mut self, focus_state: &mut FocusState, window_index: usize) {
        let (window, ..) = self.windows.remove(window_index);
        self.post_update.render();

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
    pub fn close_all_windows_except(&mut self, focus_state: &mut FocusState) {
        for index in (0..self.windows.len()).rev() {
            if self.windows[index]
                .0
                .get_window_class()
                .map(|class| class != "theme_viewer" && class != "profiler" && class != "network") // HACK: don't hardcode
                .unwrap_or(true)
            {
                self.close_window(focus_state, index);
            }
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
