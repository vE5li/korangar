#![allow(incomplete_features)]
#![feature(auto_traits)]
#![feature(let_chains)]
#![feature(negative_impls)]
// #![feature(option_zip)]
// #![feature(type_changing_struct_update)]
#![feature(macro_metavar_expr)]
#![feature(impl_trait_in_bindings)]

pub mod application;
pub mod components;
pub mod element;
pub mod event;
pub mod layout;
pub mod theme;
pub mod window;

// Re-export self as korangar_interface so we can use proc macros in this crate.
extern crate self as korangar_interface;

use application::{Appli, PositionTrait, SizeTrait, WindowCache};
use element::id::{ElementId, ElementIdGenerator};
use event::EventQueue;
// Re-export proc macros.
pub use interface_macros::{button, collapsable};
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
use layout::Layout;
use option_ext::OptionExt;
use rust_state::Context;
use window::store::WindowStore;
use window::{CustomWindow, PrototypeWindow, WindowTrait};

pub mod prelude {
    pub use interface_macros::{button, collapsable, scroll_view, state_button, text, text_box, window};

    pub use crate::components::button::ButtonThemePathExt;
    pub use crate::components::collapsable::CollapsableThemePathExt;
    pub use crate::components::state_button::StateButtonThemePathExt;
    pub use crate::components::text::TextThemePathExt;
    pub use crate::components::text_box::TextBoxThemePathExt;
    // TODO: Should this really be here?
    pub use crate::layout::HeightBound;
    pub use crate::theme::ThemePathGetter;
    pub use crate::window::WindowThemePathExt;
}

pub struct Interface<App>
where
    App: Appli,
{
    windows: Vec<Box<dyn WindowTrait<App>>>,
    window_cache: App::Cache,
    available_space: App::Size,

    generator: ElementIdGenerator,
    window_store: WindowStore,
    focused_element: Option<ElementId>,
    event_queue: EventQueue<App>,
}

impl<App> Interface<App>
where
    App: Appli,
{
    pub fn new(available_space: App::Size) -> Self {
        let window_cache = App::Cache::create();

        Self {
            windows: Vec::new(),
            window_cache,
            available_space,
            generator: ElementIdGenerator::new(),
            window_store: WindowStore::default(),
            focused_element: None,
            event_queue: EventQueue::new(),
        }
    }

    // #[cfg_attr(feature = "debug", korangar_debug::profile("update user
    // interface"))] pub fn update(&mut self, application: &App, font_loader:
    // App::FontLoader, focus_state: &mut FocusState<App>) -> (bool, bool) {
    //     for (window, post_update) in &mut self.windows {
    //         #[cfg(feature = "debug")]
    //         profile_block!("update window");
    //
    //         if let Some(change_event) = window.update() {
    //             Self::handle_change_event(&mut self.post_update, post_update,
    // change_event);         }
    //     }
    //
    //     let mut restore_focus = false;
    //
    //     for (window_index, (window, post_update)) in
    // self.windows.iter_mut().enumerate() {         if
    // self.post_update.needs_resolve() || post_update.take_resolve() {
    //             #[cfg(feature = "debug")]
    //             profile_block!("resolve window");
    //
    //             let (_position, previous_size) = window.get_area();
    //             let kind = window.get_theme_kind();
    //             let theme = application.get_theme(kind);
    //
    //             let new_size = window.resolve(font_loader.clone(), application,
    // theme, self.available_space);
    //
    //             // should only ever be the last window
    //             if let Some(focused_index) = focus_state.focused_window()
    //                 && focused_index == window_index
    //             {
    //                 restore_focus = true;
    //             }
    //
    //             // If the window got smaller, we need to re-render the entire
    // interface.             // If it got bigger, we can just draw over the
    // previous frame.             match previous_size.width() >
    // new_size.width() || previous_size.height() > new_size.height() {
    //                 true => self.post_update.render(),
    //                 false => post_update.render(),
    //             }
    //         }
    //     }
    //
    //     if restore_focus {
    //         self.restore_focus(focus_state);
    //     }
    //
    //     if self.post_update.take_resolve() {
    //         self.post_update.render();
    //     }
    //
    //     if !self.post_update.needs_render() {
    //         // We profile this block rather than the flag function itself because
    // it calls         // itself recursively
    //         #[cfg(feature = "debug")]
    //         profile_block!("flag render windows");
    //
    //         self.flag_render_windows(application, 0, None);
    //     }
    //
    //     let render_interface = self.post_update.needs_render();
    //     let render_window = self.post_update.needs_render() |
    // self.windows.iter().any(|(_window, post_update)| post_update.needs_render());
    //
    //     (render_interface, render_window)
    // }

    pub fn update_window_size(&mut self, screen_size: App::Size) {
        self.available_space = screen_size;
        // self.post_update.resolve();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn move_window_to_top(&mut self, window_index: usize) -> usize {
        let window = self.windows.remove(window_index);
        let new_window_index = self.windows.len();

        self.windows.push(window);

        new_window_index
    }

    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn left_click_element(&mut self, hovered_element: &ElementCell<App>,
    // window_index: usize) -> Vec<ClickAction<App>> {     let (_, post_update)
    // = &mut self.windows[window_index];     let mut resolve = false;
    //
    //     let action = hovered_element.borrow_mut().left_click(&mut resolve); //
    // TODO: add same change_event check as for input character ?
    //
    //     if resolve {
    //         post_update.resolve();
    //     }
    //
    //     action
    // }
    //
    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn right_click_element(&mut self, hovered_element: &ElementCell<App>,
    // window_index: usize) -> Vec<ClickAction<App>> {     let (_, post_update)
    // = &mut self.windows[window_index];     let mut resolve = false;
    //
    //     let action = hovered_element.borrow_mut().right_click(&mut resolve); //
    // TODO: add same change_event check as for input character ?
    //
    //     if resolve {
    //         post_update.resolve();
    //     }
    //
    //     action
    // }
    //
    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn drag_element(&mut self, element: &ElementCell<App>, _window_index:
    // usize, mouse_delta: App::Position) {     //let (_window, post_update) =
    // &mut self.windows[window_index];
    //
    //     if let Some(change_event) = element.borrow_mut().drag(mouse_delta) {
    //         // TODO: Use the window post_update here (?)
    //         Self::handle_change_event(&mut self.post_update, &mut
    // PostUpdate::new(), change_event);     }
    // }
    //
    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn scroll_element(&mut self, element: &ElementCell<App>, window_index:
    // usize, scroll_delta: f32) {     let (_, post_update) = &mut
    // self.windows[window_index];
    //
    //     if let Some(change_event) = element.borrow_mut().scroll(scroll_delta) {
    //         Self::handle_change_event(&mut self.post_update, post_update,
    // change_event);     }
    // }

    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn input_character_element(
    //     &mut self,
    //     element: &ElementCell<App>,
    //     window_index: usize,
    //     character: char,
    // ) -> (bool, Vec<ClickAction<App>>) {
    //     let (_, post_update) = &mut self.windows[window_index];
    //     let mut propagated_actions = Vec::new();
    //
    //     let (key_handled, actions) =
    // element.borrow_mut().input_character(character);     for action in
    // actions {         match action {
    //             ClickAction::ChangeEvent(change_event) =>
    // Self::handle_change_event(&mut self.post_update, post_update, change_event),
    //             other => propagated_actions.push(other),
    //         }
    //     }
    //
    //     (key_handled, propagated_actions)
    // }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn move_window(&mut self, window_index: usize, offset: App::Position) {
        if let Some((window_class, anchor)) = self.windows[window_index].offset(self.available_space, offset) {
            self.window_cache.update_anchor(window_class, anchor);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn resize_window(&mut self, application: &App, window_index: usize, growth: App::Size) {
        let window = &mut self.windows[window_index];

        let (window_class, new_size) = window.resize(self.available_space, growth);

        if let Some(window_class) = window_class {
            self.window_cache.update_size(window_class, new_size);
        }
    }

    // #[cfg_attr(feature = "debug", korangar_debug::profile("render user
    // interface"))] pub fn render(
    //     &mut self,
    //     renderer: &App::Renderer,
    //     application: &App,
    //     hovered_element: Option<ElementCell<App>>,
    //     focused_element: Option<ElementCell<App>>,
    //     mouse_mode: &App::MouseInputMode,
    // ) {
    //     let hovered_element = hovered_element.map(|element| unsafe {
    // &*element.as_ptr() });     let focused_element =
    // focused_element.map(|element| unsafe { &*element.as_ptr() });
    //
    //     for (index, (window, post_update)) in self.windows.iter_mut().enumerate()
    // {         if post_update.take_render() || self.post_update.needs_render()
    // {             #[cfg(feature = "debug")]
    //             profile_block!("render window");
    //
    //             let kind = window.get_theme_kind();
    //             let theme = application.get_theme(kind);
    //
    //             window.render(renderer, application, theme, hovered_element,
    // focused_element, mouse_mode);
    //
    //             if mouse_mode.is_moving_window(index) {
    //                 window.render_anchors(renderer, theme, self.available_space);
    //             }
    //         }
    //     }
    // }

    #[cfg_attr(feature = "debug", korangar_debug::profile("check window exists"))]
    fn window_exists(&self, window_class: Option<&str>) -> bool {
        match window_class {
            Some(window_class) => self
                .windows
                .iter()
                .any(|window| window.get_window_class().is_some_and(|class| class == window_class)),
            None => false,
        }
    }

    fn open_new_window(&mut self, window: impl WindowTrait<App> + 'static) {
        // TODO: `get_window_class` is already implemented on the prototype window,
        // should we really re-implement it for the Window trait too?
        if let Some(window_class) = window.get_window_class() {
            let (anchor, size) = window.get_layout();
            self.window_cache.register_window(window_class, anchor, size);
        }

        self.windows.push(Box::new(window));
        // focus_state.set_focused_window(self.windows.len() - 1);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_window<T>(&mut self, state: &Context<App>, window: T)
    where
        T: CustomWindow<App> + 'static,
    {
        if !self.window_exists(T::window_class()) {
            let window = window.to_window(state, &self.window_cache, self.available_space);
            self.open_new_window(window);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_prototype_window<T>(&mut self, application: &App, window_path: impl rust_state::Path<App, T>)
    where
        T: PrototypeWindow<App>,
    {
        if !self.window_exists(T::window_class()) {
            let window = T::to_window(window_path, &self.window_cache, application, self.available_space);
            self.open_new_window(window);
        }
    }

    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn open_popup(
    //     &mut self,
    //     element: ElementCell<App>,
    //     position_tracker: Tracker<App::Position>,
    //     size_tracker: Tracker<App::Size>,
    //     window_index: usize,
    // ) {
    //     let entry = &mut self.windows[window_index];
    //     entry.0.open_popup(element, position_tracker, size_tracker);
    //     entry.1.resolve();
    // }

    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn close_popup(&mut self, window_index: usize) {
    //     let entry = &mut self.windows[window_index];
    //     entry.0.close_popup();
    //     entry.1.render();
    // }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_window(&mut self, window_index: usize) {
        self.windows.remove(window_index);
    }

    // pub fn get_window(&self, window_index: usize) -> &dyn Window<App> {
    //     &self.windows[window_index].0
    // }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_window_with_class(&mut self, window_class: &str) {
        if let Some(index_from_back) = self
            .windows
            .iter()
            .rev()
            .map(|window| window.get_window_class())
            .position(|class_option| class_option.contains(&window_class))
        {
            let index = self.windows.len() - 1 - index_from_back;

            self.close_window(index);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_all_windows_except(&mut self) {
        for index in (0..self.windows.len()).rev() {
            if self.windows[index]
                .get_window_class()
                .map(|class| class != "theme_viewer" && class != "profiler" && class != "network") // HACK: don't hardcode
                .unwrap_or(true)
            {
                self.close_window(index);
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn do_layouts<'a>(&'a mut self, state: &'a Context<App>, mouse_position: App::Position) -> BuiltUi<'a, App> {
        let mut is_ui_hovered = false;

        let layouts = self
            .windows
            .iter()
            .map(|window| {
                let mut layout = Layout::new(mouse_position, self.focused_element, !is_ui_hovered);

                window.do_layout(
                    state,
                    &self.window_store,
                    &mut self.generator,
                    &mut layout,
                    App::Position::new(100.0, 200.0),
                );

                is_ui_hovered |= layout.is_hovered();

                layout
            })
            .collect::<Vec<_>>();

        BuiltUi {
            layouts,
            focused_element: &mut self.focused_element,
            event_queue: &mut self.event_queue,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn process_events(&mut self) -> Vec<App::Event> {
        let mut application_events = Vec::new();

        for event in self.event_queue.drain() {
            match event {
                event::Event::FocusNext => todo!(),
                event::Event::FocusPrevious => todo!(),
                event::Event::Application(application_event) => application_events.push(application_event),
            }
        }

        application_events
    }

    // #[cfg_attr(feature = "debug", korangar_debug::profile("get first focused
    // element"))] pub fn first_focused_element(&self) {
    //     if self.windows.is_empty() {
    //         return;
    //     }
    //
    //     let window_index = self.windows.len() - 1;
    //     let element = self.windows.last().unwrap().0.first_focused_element();
    //
    //     focus_state.set_focused_element(element, window_index);
    // }
    //
    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn restore_focus(&self, focus_state: &mut FocusState<App>) {
    //     if self.windows.is_empty() {
    //         return;
    //     }
    //
    //     let window_index = self.windows.len() - 1;
    //     let element = self.windows.last().unwrap().0.restore_focus();
    //
    //     focus_state.set_focused_element(element, window_index);
    // }
    //
    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn focus_window_with_class(&self, focus_state: &mut FocusState<App>,
    // window_class: &str) {     if let Some(index) = self
    //         .windows
    //         .iter()
    //         .map(|(window, ..)| window.get_window_class())
    //         .position(|class_option| class_option.contains(&window_class))
    //     {
    //         let element = self.windows[index].0.first_focused_element();
    //         focus_state.set_focused_element(element, index);
    //     }
    // }
}

pub struct BuiltUi<'a, App: Appli> {
    layouts: Vec<Layout<'a, App>>,
    focused_element: &'a mut Option<ElementId>,
    event_queue: &'a mut EventQueue<App>,
}

impl<App: Appli> BuiltUi<'_, App> {
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render(&mut self, renderer: &App::Renderer) {
        self.layouts.iter_mut().rev().for_each(|layout| {
            layout.render(renderer);
        });
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn click(&mut self, state: &Context<App>, click_position: App::Position) {
        // TODO: Rework all of this. We need more granular control over what was clicked
        // to unfocus correctly.
        let mut ui_clicked = false;

        for layout in &self.layouts {
            if layout.do_click(state, self.event_queue, self.focused_element, click_position) {
                ui_clicked = true;
                break;
            }
        }

        if !ui_clicked {
            *self.focused_element = None;
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn scroll(&self, mouse_position: App::Position, delta: f32) {
        for layout in &self.layouts {
            if layout.do_scroll(mouse_position, delta) {
                break;
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn input_characters(&self, state: &Context<App>, characters: &[char]) {
        for layout in &self.layouts {
            for character in characters {
                layout.input_character(state, *character);
            }
        }
    }
}
