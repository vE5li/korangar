#![allow(incomplete_features)]
#![feature(auto_traits)]
#![feature(let_chains)]
#![feature(negative_impls)]
// #![feature(option_zip)]
// #![feature(type_changing_struct_update)]
#![feature(macro_metavar_expr)]
#![feature(impl_trait_in_bindings)]
#![feature(unsafe_cell_access)]
#![feature(associated_type_defaults)]
#![feature(impl_trait_in_assoc_type)]

pub mod application;
pub mod components;
pub mod element;
pub mod event;
pub mod layout;
pub mod theme;
pub mod window;

// Re-export self as korangar_interface so we can use proc macros in this crate.
extern crate self as korangar_interface;

use std::any::Any;

use application::{Appli, PositionTrait, SizeTrait, WindowCache};
use element::id::{ElementId, ElementIdGenerator};
use element::store::ElementStore;
use element::{Element, ElementBox};
use event::EventQueue;
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
use layout::area::Area;
use layout::{Layout, Resolver};
use option_ext::OptionExt;
use rust_state::Context;
use window::store::WindowStore;
use window::{Anchor, CustomWindow, DisplayInformation, PrototypeWindow, WindowData, WindowTrait};

pub mod prelude {
    // Re-export proc macros.
    pub use interface_component_macros::create_component_macro;
    pub use interface_components::*;

    pub use crate::components::button::ButtonThemePathExt;
    pub use crate::components::collapsable::CollapsableThemePathExt;
    pub use crate::components::state_button::StateButtonThemePathExt;
    pub use crate::components::text::TextThemePathExt;
    pub use crate::components::text_box::TextBoxThemePathExt;
    pub use crate::event::Toggle;
    // TODO: Should this really be here?
    pub use crate::layout::HeightBound;
    pub use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    pub use crate::selector_helpers::*;
    pub use crate::theme::ThemePathGetter;
    pub use crate::window::WindowThemePathExt;
}

// TODO: Move
pub mod selector_helpers {
    use std::cell::UnsafeCell;
    use std::fmt::Display;

    use rust_state::{Path, Selector};

    use crate::application::Appli;
    use crate::element::ElementDisplay;

    pub struct PartialEqDisplaySelector<P, T> {
        path: P,
        last_value: UnsafeCell<Option<T>>,
        text: UnsafeCell<String>,
    }

    impl<P, T> PartialEqDisplaySelector<P, T> {
        pub fn new(path: P) -> Self {
            Self {
                path,
                last_value: UnsafeCell::default(),
                text: UnsafeCell::default(),
            }
        }
    }

    impl<App, P, T> Selector<App, String> for PartialEqDisplaySelector<P, T>
    where
        App: Appli,
        P: Path<App, T>,
        T: Clone + PartialEq + Display + 'static,
    {
        fn select<'a>(&'a self, state: &'a App) -> Option<&'a String> {
            // SAFETY
            // `unnwrap` is safe here because the bound of `P` specifies a safe path.
            let value = self.path.follow(state).unwrap();

            unsafe {
                let last_value = &mut *self.last_value.get();

                if last_value.is_none() || last_value.as_ref().is_some_and(|last| last != value) {
                    *self.text.get() = value.to_string();
                    *last_value = Some(value.clone());
                }
            }

            unsafe { Some(self.text.as_ref_unchecked()) }
        }
    }

    pub struct ElementDisplaySelector<P, T> {
        path: P,
        last_value: UnsafeCell<Option<T>>,
        text: UnsafeCell<String>,
    }

    impl<P, T> ElementDisplaySelector<P, T> {
        pub fn new(path: P) -> Self {
            Self {
                path,
                last_value: UnsafeCell::default(),
                text: UnsafeCell::default(),
            }
        }
    }

    impl<App, P, T> Selector<App, String> for ElementDisplaySelector<P, T>
    where
        App: Appli,
        P: Path<App, T>,
        T: ElementDisplay,
    {
        fn select<'a>(&'a self, state: &'a App) -> Option<&'a String> {
            // SAFETY
            // `unnwrap` is safe here because the bound of `P` specifies a safe path.
            let value = self.path.follow(state).unwrap();

            unsafe {
                let last_value = &mut *self.last_value.get();

                if last_value.is_none() || last_value.as_ref().is_some_and(|last| last != value) {
                    *self.text.get() = value.element_display();
                    *last_value = Some(value.clone());
                }
            }

            unsafe { Some(self.text.as_ref_unchecked()) }
        }
    }
}

// TODO: This will likely be renamed + Moved
pub enum MouseMode {
    Default,
    MovingWindow { window_id: u64 },
    ResizingWindow { window_id: u64 },
}

struct WindowWrapper<App>
where
    App: Appli,
{
    window: Box<dyn WindowTrait<App>>,
    data: WindowData<App>,
    display_information: DisplayInformation<App>,
}

pub struct Interface<App>
where
    App: Appli,
{
    windows: Vec<WindowWrapper<App>>,
    window_cache: App::Cache,
    window_size: App::Size,

    generator: ElementIdGenerator,
    window_store: WindowStore,
    focused_element: Option<ElementId>,
    mouse_mode: MouseMode,
    event_queue: EventQueue<App>,
    overlay_element: Option<(ElementBox<App>, ElementStore)>,

    next_window_id: u64,
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
            window_size: available_space,

            generator: ElementIdGenerator::new(),
            window_store: WindowStore::default(),
            focused_element: None,
            mouse_mode: MouseMode::Default,
            event_queue: EventQueue::default(),
            overlay_element: None,

            next_window_id: 0,
        }
    }

    pub fn update_window_size(&mut self, screen_size: App::Size) {
        self.window_size = screen_size;
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
    pub fn handle_drag(&mut self, delta: App::Size) {
        match self.mouse_mode {
            MouseMode::Default => {}
            MouseMode::MovingWindow { window_id } => {
                if let Some(wrapper) = self.windows.iter_mut().find(|window| window.data.id == window_id) {
                    let new_position = App::Position::new(
                        wrapper.display_information.real_position.left() + delta.width(),
                        wrapper.display_information.real_position.top() + delta.height(),
                    );

                    wrapper
                        .data
                        .anchor
                        .update(self.window_size, new_position, wrapper.display_information.real_size);

                    if let Some(window_class) = wrapper.window.get_window_class() {
                        self.window_cache.update_anchor(window_class, wrapper.data.anchor);
                    }
                }
            }
            MouseMode::ResizingWindow { window_id } => {
                if let Some(wrapper) = self.windows.iter_mut().find(|window| window.data.id == window_id) {
                    wrapper.data.size = App::Size::new(
                        wrapper.display_information.real_size.width() + delta.width(),
                        wrapper.display_information.real_size.height() + delta.height(),
                    );

                    if let Some(window_class) = wrapper.window.get_window_class() {
                        self.window_cache.update_size(window_class, wrapper.data.size);
                    }
                }
            }
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

    pub fn is_window_with_class_open(&self, window_class: App::WindowClass) -> bool {
        self.windows
            .iter()
            .any(|wrapper| wrapper.window.get_window_class().is_some_and(|class| class == window_class))
    }

    fn open_new_window(&mut self, window: impl WindowTrait<App> + 'static) {
        let id = self.next_window_id;
        // TODO: Actual logic to wrap around and adjust all window IDs.
        self.next_window_id = self.next_window_id.wrapping_add(1);

        let window_class = window.get_window_class();

        let (anchor, size) = match window_class.and_then(|window_class| self.window_cache.get_window_state(window_class)) {
            Some(saved_state) => saved_state,
            None => {
                let anchor = Anchor::default();
                let size = App::Size::new(0.0, 500.0);

                if let Some(window_class) = window_class {
                    self.window_cache.register_window(window_class, anchor, size);
                }

                (anchor, size)
            }
        };

        self.windows.insert(0, WindowWrapper {
            window: Box::new(window),
            data: WindowData { id, anchor, size },
            display_information: DisplayInformation {
                real_position: App::Position::new(0.0, 0.0),
                real_size: size,
                display_height: 0.0,
            },
        });
        // focus_state.set_focused_window(self.windows.len() - 1);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_window<T>(&mut self, window: T)
    where
        T: CustomWindow<App> + 'static,
    {
        if !T::window_class().is_some_and(|window_class| self.is_window_with_class_open(window_class)) {
            let window = window.to_window();
            self.open_new_window(window);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_prototype_window<T>(&mut self, window_path: impl rust_state::Path<App, T>)
    where
        T: PrototypeWindow<App>,
    {
        if !T::window_class().is_some_and(|window_class| self.is_window_with_class_open(window_class)) {
            let window = T::to_window(window_path);
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

    // pub fn get_window(&self, window_index: usize) -> &dyn Window<App> {
    //     &self.windows[window_index].0
    // }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_window_with_class(&mut self, window_class: App::WindowClass) {
        if let Some(index_from_back) = self
            .windows
            .iter()
            .rev()
            .map(|wrapper| wrapper.window.get_window_class())
            .position(|class_option| class_option.contains(&window_class))
        {
            let index = self.windows.len() - 1 - index_from_back;

            self.windows.remove(index);
        }
    }

    // #[cfg_attr(feature = "debug", korangar_debug::profile)]
    // pub fn close_window_with_id(&mut self, window_id: u64) {
    //     if let Some(index) = self.windows.iter().position(|(_, window_data)|
    // window_data.id == window_id) {         self.close_window(index);
    //     }
    // }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_all_windows_except(&mut self, exceptions: &[App::WindowClass]) {
        for index in (0..self.windows.len()).rev() {
            if self.windows[index]
                .window
                .get_window_class()
                .map(|class| !exceptions.contains(&class))
                .unwrap_or(true)
            {
                self.windows.remove(index);
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn do_layouts<'a>(&'a mut self, state: &'a Context<App>, mouse_position: App::Position) -> BuiltUi<'a, App> {
        let mut is_ui_hovered = false;

        if let Some((element, store)) = &mut self.overlay_element {
            let available_area = Area {
                x: 200.0,
                y: 200.0,
                width: 400.0,
                height: 400.0,
            };

            let mut resolver = Resolver::new(available_area, 0.0);

            element.make_layout(state, store, &mut self.generator, &mut resolver);
        }

        self.windows.iter_mut().for_each(|wrapper| {
            wrapper.display_information = wrapper.window.make_layout(
                state,
                &mut self.window_store,
                &wrapper.data,
                &mut self.generator,
                self.window_size,
            );
        });

        let overlay_layout = self.overlay_element.as_ref().map(|(element, store)| {
            let mut layout = Layout::new(mouse_position, self.focused_element, !is_ui_hovered);

            element.create_layout(state, store, &(), &mut layout);

            is_ui_hovered |= layout.is_hovered();

            layout
        });

        let mut layouts = self
            .windows
            .iter()
            .map(|wrapper| {
                let mut layout = Layout::new(mouse_position, self.focused_element, !is_ui_hovered);

                wrapper.window.do_layout(state, &self.window_store, &wrapper.data, &mut layout);

                is_ui_hovered |= layout.is_hovered();

                layout
            })
            .collect::<Vec<_>>();

        if let Some(layout) = overlay_layout {
            layouts.insert(0, layout);
        }

        BuiltUi {
            layouts,
            focused_element: &mut self.focused_element,
            mouse_mode: &mut self.mouse_mode,
            event_queue: &mut self.event_queue,
        }
    }

    pub fn render_overlay(&self, renderer: &App::Renderer, anchor_color: App::Color, closest_anchor_color: App::Color) {
        if let MouseMode::MovingWindow { window_id } = self.mouse_mode {
            if let Some(wrapper) = self.windows.iter().find(|wrapper| wrapper.data.id == window_id) {
                wrapper
                    .data
                    .anchor
                    .render_screen_anchors(renderer, anchor_color, closest_anchor_color, self.window_size);

                let window_display_size = App::Size::new(
                    wrapper.display_information.real_size.width(),
                    wrapper.display_information.display_height,
                );

                wrapper.data.anchor.render_window_anchors(
                    renderer,
                    anchor_color,
                    closest_anchor_color,
                    wrapper.display_information.real_position,
                    window_display_size,
                );
            }
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
                event::Event::OpenOverlay(element) => self.overlay_element = Some((element, ElementStore::root(&mut self.generator))),
                event::Event::CloseWindow { window_id } => {
                    if let Some(index) = self.windows.iter().position(|wrapper| wrapper.data.id == window_id) {
                        self.windows.remove(index);
                    }
                }
                event::Event::CloseOverlay => self.overlay_element = None,
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
    mouse_mode: &'a mut MouseMode,
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
            if layout.do_click(state, self.event_queue, self.focused_element, self.mouse_mode, click_position) {
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
