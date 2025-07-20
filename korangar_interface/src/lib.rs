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

use std::collections::BTreeMap;

use application::{Application, ClipTrait, FontSizeTrait, PositionTrait, RenderLayer, SizeTrait, WindowCache};
use element::id::{ElementId, ElementIdGenerator};
use element::store::ElementStore;
use element::{Element, ElementBox};
use event::EventQueue;
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
use layout::area::Area;
use layout::{Layout, MouseButton, Resolver};
use option_ext::OptionExt;
use rust_state::Context;
use theme::ThemePathGetter;
use window::store::WindowStore;
use window::{Anchor, CustomWindow, DisplayInformation, StateWindow, WindowData, WindowThemePathExt, WindowTrait};

pub mod prelude {
    // Re-export proc macros.
    pub use interface_component_macros::create_component_macro;
    pub use interface_components::*;

    pub use crate::components::button::ButtonThemePathExt;
    pub use crate::components::collapsable::CollapsableThemePathExt;
    pub use crate::components::drop_down::DropDownThemePathExt;
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

    use crate::application::Application;
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
        App: Application,
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
        App: Application,
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
    App: Application,
{
    window: Box<dyn WindowTrait<App>>,
    data: WindowData<App>,
    display_information: DisplayInformation<App>,
}

pub struct Interface<'a, App>
where
    App: Application,
{
    windows: Vec<WindowWrapper<App>>,
    window_cache: App::Cache,
    window_size: App::Size,

    generator: ElementIdGenerator,
    window_store: WindowStore,
    focused_element: Option<ElementId>,
    mouse_mode: MouseMode,
    event_queue: EventQueue<App>,
    overlay_element: Option<(ElementBox<App>, ElementStore, App::Position, App::Size)>,

    /// Cached window layouts. This is mostly an optimization to avoid
    /// re-allocating the every frame but also serves to store some information
    /// between frames like the tooltip timers.
    window_layouts: BTreeMap<u64, Layout<'a, App>>,
    /// Cached overlay layout. This is mostly an optimization to avoid
    /// re-allocating the every frame but also serves to store some information
    /// between frames like the tooltip timers.
    overlay_layout: Option<Layout<'a, App>>,

    next_window_id: u64,
}

impl<App> Interface<'static, App>
where
    App: Application,
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

            window_layouts: BTreeMap::new(),
            overlay_layout: None,

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

                    wrapper.data.anchor.update(
                        self.window_size,
                        new_position,
                        wrapper.display_information.real_size,
                        wrapper.display_information.display_height,
                    );

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
                let size = App::Size::new(0.0, f32::MAX);

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
    }

    pub fn reset_mouse_mode(&mut self) {
        self.mouse_mode = MouseMode::Default;
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
    pub fn open_state_window<T>(&mut self, window_path: impl rust_state::Path<App, T>)
    where
        T: StateWindow<App>,
    {
        if !T::window_class().is_some_and(|window_class| self.is_window_with_class_open(window_class)) {
            let window = T::to_window(window_path);
            self.open_new_window(window);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_state_window_mut<T>(&mut self, window_path: impl rust_state::Path<App, T>)
    where
        T: StateWindow<App>,
    {
        if !T::window_class().is_some_and(|window_class| self.is_window_with_class_open(window_class)) {
            let window = T::to_window_mut(window_path);
            self.open_new_window(window);
        }
    }

    fn remove_window(&mut self, index: usize) {
        // Remove the cached window layout to avoid growing the cache indefinitely.
        self.window_layouts.remove(&self.windows[index].data.id);
        self.windows.remove(index);
    }

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
            self.remove_window(index);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_all_windows(&mut self) {
        for index in (0..self.windows.len()).rev() {
            self.remove_window(index);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_all_windows_except(&mut self, exceptions: &[App::WindowClass]) {
        for index in (0..self.windows.len()).rev() {
            if self.windows[index]
                .window
                .get_window_class()
                .map(|class| !exceptions.contains(&class))
                .unwrap_or(true)
            {
                self.remove_window(index);
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn do_layouts<'a>(&'a mut self, state: &'a Context<App>, mouse_position: App::Position) -> BuiltUi<'a, App> {
        let mut is_ui_hovered = false;

        if let Some((element, store, position, size)) = &mut self.overlay_element {
            let available_area = Area {
                x: position.left(),
                y: position.top(),
                width: size.width(),
                height: size.height(),
            };

            let mut resolver = Resolver::new(available_area, 0.0);

            element.create_layout_info(state, store, &mut self.generator, &mut resolver);
        }

        // SAFETY:
        //
        // This is safe as long as the `Drop` implementation of the `BuiltUi` clears the
        // `Layout`s, removing any references with the lifetime 'a from the
        // struct. If drop does not clear the layout or the clear implementation
        // of drop is incorrect we will end up with dangling references.
        let this = unsafe { std::mem::transmute::<&'a mut Interface<'static, App>, &'a mut Interface<'a, App>>(self) };

        this.windows.iter_mut().for_each(|wrapper| {
            wrapper.display_information = wrapper.window.create_layout_info(
                state,
                &mut this.window_store,
                &mut wrapper.data,
                &mut this.generator,
                this.window_size,
            );
        });

        if let Some((element, store, ..)) = &this.overlay_element {
            let layout = this.overlay_layout.get_or_insert_default();
            layout.update(mouse_position, this.focused_element, !is_ui_hovered);

            element.layout_element(state, store, &(), layout);

            is_ui_hovered |= layout.is_hovered();
        }

        this.windows.iter().for_each(|wrapper| {
            let layout = this.window_layouts.entry(wrapper.data.id).or_default();
            layout.update(mouse_position, this.focused_element, !is_ui_hovered);

            wrapper.window.do_layout(state, &this.window_store, &wrapper.data, layout);

            is_ui_hovered |= layout.is_hovered();
        });

        BuiltUi {
            window_layouts: &mut this.window_layouts,
            overlay_layout: &mut this.overlay_layout,
            focused_element: &mut this.focused_element,
            mouse_mode: &mut this.mouse_mode,
            event_queue: &mut this.event_queue,
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
                event::Event::OpenOverlay { element, position, size } => {
                    self.overlay_element = Some((element, ElementStore::root(&mut self.generator), position, size))
                }
                event::Event::CloseWindow { window_id } => {
                    if let Some(index) = self.windows.iter().position(|wrapper| wrapper.data.id == window_id) {
                        self.windows.remove(index);
                    }
                }
                event::Event::CloseOverlay => {
                    self.overlay_element = None;
                    self.overlay_layout = None;
                }
            }
        }

        application_events
    }
}

pub struct BuiltUi<'a, App: Application> {
    window_layouts: &'a mut BTreeMap<u64, Layout<'a, App>>,
    overlay_layout: &'a mut Option<Layout<'a, App>>,
    focused_element: &'a mut Option<ElementId>,
    mouse_mode: &'a mut MouseMode,
    event_queue: &'a mut EventQueue<App>,
}

impl<App: Application> BuiltUi<'_, App> {
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render(&mut self, renderer: &App::Renderer) {
        // FIX: Window order.
        // Most likely we want to include another vector that keeps the ids of the
        // windows in order.
        self.window_layouts.values_mut().for_each(|layout| {
            layout.render(renderer);
        });

        if let Some(layout) = &mut self.overlay_layout {
            layout.render(renderer);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn click(&mut self, state: &Context<App>, click_position: App::Position, mouse_button: MouseButton) {
        // TODO: Rework all of this. We need more granular control over what was clicked
        // to unfocus correctly.
        let mut ui_clicked = false;

        if let Some(layout) = &self.overlay_layout {
            if layout.do_click(
                state,
                self.event_queue,
                self.focused_element,
                self.mouse_mode,
                click_position,
                mouse_button,
            ) {
                ui_clicked = true;
            }
        }

        if !ui_clicked {
            for layout in self.window_layouts.values() {
                if layout.do_click(
                    state,
                    self.event_queue,
                    self.focused_element,
                    self.mouse_mode,
                    click_position,
                    mouse_button,
                ) {
                    ui_clicked = true;
                    break;
                }
            }
        }

        if !ui_clicked {
            *self.focused_element = None;
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn scroll(&self, mouse_position: App::Position, delta: f32) {
        if let Some(layout) = &self.overlay_layout {
            if layout.do_scroll(mouse_position, delta) {
                return;
            }
        }

        for layout in self.window_layouts.values() {
            if layout.do_scroll(mouse_position, delta) {
                break;
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn input_characters(&self, state: &Context<App>, characters: &[char]) {
        if let Some(layout) = &self.overlay_layout {
            for character in characters {
                layout.input_character(state, *character);
            }
        }

        for layout in self.window_layouts.values() {
            for character in characters {
                layout.input_character(state, *character);
            }
        }
    }
}

impl<App: Application> Drop for BuiltUi<'_, App> {
    fn drop(&mut self) {
        self.window_layouts.values_mut().for_each(|layout| layout.clear());

        if let Some(layout) = self.overlay_layout {
            layout.clear();
        }
    }
}
