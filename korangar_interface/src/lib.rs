#![feature(auto_traits)]
#![feature(generic_const_exprs)]
#![feature(let_chains)]
#![feature(negative_impls)]
#![feature(option_zip)]
#![feature(specialization)]
#![feature(type_changing_struct_update)]
#![allow(incomplete_features)]

pub mod application;
pub mod event;
pub mod layout;
pub mod state;
pub mod theme;
#[macro_use]
pub mod elements;
pub mod builder;
pub mod windows;

use std::marker::PhantomData;

use application::{Application, FocusState, InterfaceRenderer, SizeTrait, SizeTraitExt, WindowCache};
use elements::ElementCell;
use event::{ChangeEvent, ClickAction, HoverInformation};
// Re-export proc macros.
pub use interface_procedural::{dimension_bound, size_bound};
use option_ext::OptionExt;
use windows::{PrototypeWindow, Window};

// TODO: move this
pub type Selector = Box<dyn Fn() -> bool>;
#[allow(type_alias_bounds)]
pub type ColorSelector<App: Application> = Box<dyn Fn(&App::Theme) -> App::Color>;
#[allow(type_alias_bounds)]
pub type FontSizeSelector<App: Application> = Box<dyn Fn(&App::Theme) -> App::FontSize>;

// NOTE: To make proc macro work.
mod korangar_interface {}

pub trait ElementEvent<App>
where
    App: Application,
{
    fn trigger(&mut self) -> Vec<ClickAction<App>>;
}

impl<App, F> ElementEvent<App> for F
where
    App: Application,
    F: FnMut() -> Vec<ClickAction<App>> + 'static,
{
    fn trigger(&mut self) -> Vec<ClickAction<App>> {
        self()
    }
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

pub type Tracker<T> = Box<dyn Fn() -> Option<T>>;

pub struct Interface<App>
where
    App: Application,
{
    windows: Vec<(Window<App>, PostUpdate<PerWindow>)>,
    window_cache: App::Cache,
    available_space: App::Size,
    post_update: PostUpdate<Self>,
}

impl<App> Interface<App>
where
    App: Application,
{
    pub fn new(available_space: App::Size) -> Self {
        let window_cache = App::Cache::create();
        // NOTE: We need to initially clear the interface buffer
        let post_update = PostUpdate::new().with_render();

        Self {
            windows: Vec::new(),
            window_cache,
            available_space,
            post_update,
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

    #[cfg_attr(feature = "debug", korangar_debug::profile("update user interface"))]
    pub fn update(&mut self, application: &App, font_loader: App::FontLoader, focus_state: &mut FocusState<App>) -> (bool, bool) {
        for (window, post_update) in &mut self.windows {
            #[cfg(feature = "debug")]
            korangar_debug::profile_block!("update window");

            if let Some(change_event) = window.update() {
                Self::handle_change_event(&mut self.post_update, post_update, change_event);
            }
        }

        let mut restore_focus = false;

        for (window_index, (window, post_update)) in self.windows.iter_mut().enumerate() {
            if self.post_update.needs_resolve() || post_update.take_resolve() {
                #[cfg(feature = "debug")]
                korangar_debug::profile_block!("resolve window");

                let (_position, previous_size) = window.get_area();
                let kind = window.get_theme_kind();
                let theme = application.get_theme(kind);

                let (window_class, new_position, new_size) = window.resolve(font_loader.clone(), application, theme, self.available_space);

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
                match previous_size.width() > new_size.width() || previous_size.height() > new_size.height() {
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
            korangar_debug::profile_block!("flag render windows");

            self.flag_render_windows(application, 0, None);
        }

        let render_interface = self.post_update.needs_render();
        let render_window = self.post_update.needs_render() | self.windows.iter().any(|(_window, post_update)| post_update.needs_render());

        (render_interface, render_window)
    }

    pub fn update_window_size(&mut self, screen_size: App::Size) {
        self.available_space = screen_size;
        self.post_update.resolve();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("get hovered element"))]
    pub fn hovered_element(
        &self,
        mouse_position: App::Position,
        mouse_mode: &App::MouseInputMode,
    ) -> (Option<ElementCell<App>>, Option<usize>) {
        for (window_index, (window, _)) in self.windows.iter().enumerate().rev() {
            match window.hovered_element(mouse_position, mouse_mode) {
                HoverInformation::Element(hovered_element) => return (Some(hovered_element), Some(window_index)),
                HoverInformation::Hovered => return (None, Some(window_index)),
                HoverInformation::Missed => {}
            }
        }

        (None, None)
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn move_window_to_top(&mut self, window_index: usize) -> usize {
        let (window, post_update) = self.windows.remove(window_index);
        let new_window_index = self.windows.len();

        self.windows.push((window, post_update.with_render()));

        new_window_index
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn left_click_element(&mut self, hovered_element: &ElementCell<App>, window_index: usize) -> Vec<ClickAction<App>> {
        let (_, post_update) = &mut self.windows[window_index];
        let mut resolve = false;

        let action = hovered_element.borrow_mut().left_click(&mut resolve); // TODO: add same change_event check as for input character ?

        if resolve {
            post_update.resolve();
        }

        action
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn right_click_element(&mut self, hovered_element: &ElementCell<App>, window_index: usize) -> Vec<ClickAction<App>> {
        let (_, post_update) = &mut self.windows[window_index];
        let mut resolve = false;

        let action = hovered_element.borrow_mut().right_click(&mut resolve); // TODO: add same change_event check as for input character ?

        if resolve {
            post_update.resolve();
        }

        action
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn drag_element(&mut self, element: &ElementCell<App>, _window_index: usize, mouse_delta: App::Position) {
        //let (_window, post_update) = &mut self.windows[window_index];

        if let Some(change_event) = element.borrow_mut().drag(mouse_delta) {
            // TODO: Use the window post_update here (?)
            Self::handle_change_event(&mut self.post_update, &mut PostUpdate::new(), change_event);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn scroll_element(&mut self, element: &ElementCell<App>, window_index: usize, scroll_delta: f32) {
        let (_, post_update) = &mut self.windows[window_index];

        if let Some(change_event) = element.borrow_mut().scroll(scroll_delta) {
            Self::handle_change_event(&mut self.post_update, post_update, change_event);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn input_character_element(&mut self, element: &ElementCell<App>, window_index: usize, character: char) -> Vec<ClickAction<App>> {
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

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn move_window(&mut self, window_index: usize, offset: App::Position) {
        if let Some((window_class, position)) = self.windows[window_index].0.offset(self.available_space, offset) {
            self.window_cache.update_position(window_class, position);
        }

        self.post_update.render();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn resize_window(&mut self, application: &App, window_index: usize, growth: App::Size) {
        let (window, post_update) = &mut self.windows[window_index];

        let (_position, previous_size) = window.get_area();
        let (window_class, new_size) = window.resize(application, self.available_space, growth);

        if !previous_size.is_equal(new_size) {
            if let Some(window_class) = window_class {
                self.window_cache.update_size(window_class, new_size);
            }

            post_update.resolve();

            if previous_size.width() > new_size.width() || previous_size.height() > new_size.height() {
                self.post_update.render();
            }
        }
    }

    /// This function is solely responsible for making sure that trying to
    /// re-render a window with transparency will result in re-rendering the
    /// entire interface. This serves as a single point of truth and simplifies
    /// the rest of the code.
    fn flag_render_windows(&mut self, application: &App, start_index: usize, area: Option<(App::Position, App::Size)>) {
        for window_index in start_index..self.windows.len() {
            let needs_render = self.windows[window_index].1.needs_render();
            let is_hovering = |(position, scale)| self.windows[window_index].0.hovers_area(position, scale);

            if needs_render || area.map(is_hovering).unwrap_or(false) {
                let (position, scale) = {
                    let (window, post_update) = &mut self.windows[window_index];

                    let kind = window.get_theme_kind();
                    let theme = application.get_theme(kind);

                    if window.has_transparency(theme) {
                        self.post_update.render();
                        return;
                    }

                    post_update.render();
                    window.get_area()
                };

                self.flag_render_windows(application, window_index + 1, Some((position, scale)));
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render user interface"))]
    pub fn render(
        &mut self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &App,
        hovered_element: Option<ElementCell<App>>,
        focused_element: Option<ElementCell<App>>,
        mouse_mode: &App::MouseInputMode,
    ) {
        let hovered_element = hovered_element.map(|element| unsafe { &*element.as_ptr() });
        let focused_element = focused_element.map(|element| unsafe { &*element.as_ptr() });

        for (window, post_update) in &mut self.windows {
            if post_update.take_render() || self.post_update.needs_render() {
                #[cfg(feature = "debug")]
                korangar_debug::profile_block!("render window");

                let kind = window.get_theme_kind();
                let theme = application.get_theme(kind);

                window.render(
                    render_target,
                    renderer,
                    application,
                    theme,
                    hovered_element,
                    focused_element,
                    mouse_mode,
                );
            }
        }

        self.post_update.take_render();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("check window exists"))]
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

    fn open_new_window(&mut self, focus_state: &mut FocusState<App>, window: Window<App>) {
        self.windows.push((window, PostUpdate::new().with_resolve()));
        focus_state.set_focused_window(self.windows.len() - 1);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_window(&mut self, application: &App, focus_state: &mut FocusState<App>, prototype_window: &dyn PrototypeWindow<App>) {
        if !self.window_exists(prototype_window.window_class()) {
            let window = prototype_window.to_window(&self.window_cache, application, self.available_space);
            self.open_new_window(focus_state, window);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_popup(
        &mut self,
        element: ElementCell<App>,
        position_tracker: Tracker<App::Position>,
        size_tracker: Tracker<App::Size>,
        window_index: usize,
    ) {
        let entry = &mut self.windows[window_index];
        entry.0.open_popup(element, position_tracker, size_tracker);
        entry.1.resolve();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_popup(&mut self, window_index: usize) {
        let entry = &mut self.windows[window_index];
        entry.0.close_popup();
        entry.1.render();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_window(&mut self, focus_state: &mut FocusState<App>, window_index: usize) {
        let (window, ..) = self.windows.remove(window_index);
        self.post_update.render();

        // drop window in another thread to avoid frame drops when deallocation a large
        // amount of elements
        std::thread::spawn(move || drop(window));

        // TODO: only if tab mode
        self.restore_focus(focus_state);
    }

    pub fn get_window(&self, window_index: usize) -> &Window<App> {
        &self.windows[window_index].0
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_window_with_class(&mut self, focus_state: &mut FocusState<App>, window_class: &str) {
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

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_all_windows_except(&mut self, focus_state: &mut FocusState<App>) {
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

    #[cfg_attr(feature = "debug", korangar_debug::profile("get first focused element"))]
    pub fn first_focused_element(&self, focus_state: &mut FocusState<App>) {
        if self.windows.is_empty() {
            return;
        }

        let window_index = self.windows.len() - 1;
        let element = self.windows.last().unwrap().0.first_focused_element();

        focus_state.set_focused_element(element, window_index);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn restore_focus(&self, focus_state: &mut FocusState<App>) {
        if self.windows.is_empty() {
            return;
        }

        let window_index = self.windows.len() - 1;
        let element = self.windows.last().unwrap().0.restore_focus();

        focus_state.set_focused_element(element, window_index);
    }
}
