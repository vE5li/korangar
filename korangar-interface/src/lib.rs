#![allow(incomplete_features)]
#![feature(auto_traits)]
#![feature(negative_impls)]
#![feature(macro_metavar_expr)]
#![feature(impl_trait_in_bindings)]
#![feature(unsafe_cell_access)]
#![feature(associated_type_defaults)]
#![feature(impl_trait_in_assoc_type)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(allocator_api)]

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
use std::collections::BTreeMap;

use application::{Application, Clip, CornerDiameter, FontSize, Position, RenderLayer, Size, TextLayouter, WindowCache};
use element::ElementBox;
use element::id::{ElementId, ElementIdGenerator};
use element::store::{ElementStore, ElementStoreMut, InternalElementStore};
use event::{Event, EventQueue};
use layout::area::Area;
use layout::tooltip::TooltipTheme;
use layout::{MouseButton, ResizeMode, Resolver, WindowLayout};
use option_ext::OptionExt;
use rust_state::Context;
use theme::ThemePathGetter;
use window::store::WindowStore;
use window::{Anchor, CustomWindow, DisplayInformation, StateWindow, Window, WindowData, WindowThemePathExt};

use crate::element::id::FocusIdExt;

pub mod prelude {
    //! Prelude for implementing elements or windows. Mainly to reduce the
    //! number of `*PathExt` imports.

    // Re-export proc macros.
    pub use interface_component_macros::create_component_macro;
    pub use interface_components::*;

    pub use crate::components::button::ButtonThemePathExt;
    pub use crate::components::collapsable::CollapsableThemePathExt;
    pub use crate::components::drop_down::DropDownThemePathExt;
    pub use crate::components::field::FieldThemePathExt;
    pub use crate::components::state_button::StateButtonThemePathExt;
    pub use crate::components::text::TextThemePathExt;
    pub use crate::components::text_box::TextBoxThemePathExt;
    pub use crate::element::ErasedElement;
    pub use crate::event::{Event, EventQueue, Toggle};
    pub use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    pub use crate::layout::tooltip::TooltipThemePathExt;
    pub use crate::selector_helpers::*;
    pub use crate::theme::{ThemePathGetter, theme};
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

    pub struct ComputedSelector<F, T> {
        closure: F,
        latest_value: UnsafeCell<T>,
    }

    impl<F, T> ComputedSelector<F, T>
    where
        T: Default,
    {
        pub fn new_default(closure: F) -> Self {
            Self {
                closure,
                latest_value: UnsafeCell::new(T::default()),
            }
        }
    }

    impl<App, F, T> Selector<App, T> for ComputedSelector<F, T>
    where
        F: Fn(&App) -> T + 'static,
        T: 'static,
    {
        fn select<'a>(&'a self, state: &'a App) -> Option<&'a T> {
            let new_value = (self.closure)(state);

            let latest_value = unsafe { &mut *self.latest_value.get() };
            *latest_value = new_value;

            Some(latest_value)
        }
    }
}

// TODO: This will likely be renamed + Moved
pub enum MouseMode<App>
where
    App: Application,
{
    Default,
    MovingWindow { window_id: u64 },
    ResizingWindow { resize_mode: ResizeMode, window_id: u64 },
    Custom { mode: App::CustomMouseMode },
}

impl<App> MouseMode<App>
where
    App: Application,
{
    pub fn is_default(&self) -> bool {
        matches!(self, MouseMode::Default)
    }
}

impl<App> Clone for MouseMode<App>
where
    App: Application,
    App::CustomMouseMode: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Default => Self::Default,
            Self::MovingWindow { window_id } => Self::MovingWindow { window_id: *window_id },
            Self::ResizingWindow { resize_mode, window_id } => Self::ResizingWindow {
                resize_mode: *resize_mode,
                window_id: *window_id,
            },
            Self::Custom { mode } => Self::Custom { mode: mode.clone() },
        }
    }
}

struct WindowWrapper<App>
where
    App: Application,
{
    window: Box<dyn Window<App>>,
    data: WindowData<App>,
    display_information: DisplayInformation,
}

struct OverlayElement<App>
where
    App: Application,
{
    element: ElementBox<App>,
    store: InternalElementStore,
    position: App::Position,
    size: App::Size,
    window_id: u64,
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
    mouse_mode: MouseMode<App>,
    event_queue: EventQueue<App>,
    overlay_element: Option<OverlayElement<App>>,

    /// Cached window layouts. This is mostly an optimization to avoid
    /// re-allocating the every frame but also serves to store some information
    /// between frames like the tooltip timers.
    window_layouts: BTreeMap<u64, WindowLayout<'a, App>>,
    /// Cached overlay layout. This is mostly an optimization to avoid
    /// re-allocating the every frame but also serves to store some information
    /// between frames like the tooltip timers.
    overlay_layout: Option<WindowLayout<'a, App>>,

    next_window_id: u64,

    text_layouter: App::TextLayouter,
}

impl<App> Interface<'static, App>
where
    App: Application,
{
    pub fn new(text_layouter: App::TextLayouter, available_space: App::Size) -> Self {
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
            text_layouter,
        }
    }

    pub fn update_window_size(&mut self, screen_size: App::Size) {
        self.window_size = screen_size;
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn handle_drag(&mut self, delta: App::Size, interface_scaling: f32) {
        match self.mouse_mode {
            MouseMode::Default => {}
            MouseMode::MovingWindow { window_id } => {
                if let Some(wrapper) = self.windows.iter_mut().find(|window| window.data.id == window_id) {
                    let new_position = App::Position::new(
                        wrapper.display_information.real_area.left + delta.width(),
                        wrapper.display_information.real_area.top + delta.height(),
                    );

                    let scaled_size = App::Size::new(
                        wrapper.display_information.real_area.width * interface_scaling,
                        wrapper.display_information.display_height * interface_scaling,
                    );

                    wrapper.data.anchor.update(self.window_size, new_position, scaled_size);

                    if let Some(window_class) = wrapper.window.get_class() {
                        self.window_cache.update_anchor(window_class, wrapper.data.anchor);
                    }
                }
            }
            MouseMode::ResizingWindow { window_id, resize_mode } => {
                if let Some(wrapper) = self.windows.iter_mut().find(|window| window.data.id == window_id) {
                    let (delta_width, delta_height) = match resize_mode {
                        ResizeMode::Horizontal => (delta.width(), 0.0),
                        ResizeMode::Vertical => (0.0, delta.height()),
                        ResizeMode::Both => (delta.width(), delta.height()),
                    };

                    wrapper.data.size = App::Size::new(
                        wrapper.display_information.real_area.width + delta_width / interface_scaling,
                        wrapper.display_information.real_area.height + delta_height / interface_scaling,
                    );

                    if let Some(window_class) = wrapper.window.get_class() {
                        self.window_cache.update_size(window_class, wrapper.data.size);
                    }
                }
            }
            MouseMode::Custom { .. } => {}
        }
    }

    pub fn is_window_with_class_open(&self, window_class: App::WindowClass) -> bool {
        self.windows
            .iter()
            .any(|wrapper| wrapper.window.get_class().is_some_and(|class| class == window_class))
    }

    fn open_new_window(&mut self, window: impl Window<App> + 'static) {
        let id = self.next_window_id;
        // TODO: Actual logic to wrap around and adjust all window IDs.
        self.next_window_id = self.next_window_id.wrapping_add(1);

        let window_class = window.get_class();

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

        self.windows.push(WindowWrapper {
            window: Box::new(window),
            data: WindowData { id, anchor, size },
            display_information: DisplayInformation {
                real_area: Area {
                    left: 0.0,
                    top: 0.0,
                    width: size.width(),
                    height: size.height(),
                },
                display_height: 0.0,
            },
        });
    }

    pub fn get_mouse_mode(&self) -> &MouseMode<App> {
        &self.mouse_mode
    }

    pub fn has_focus(&self) -> bool {
        self.focused_element.is_some()
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
    pub fn close_top_window(&mut self, state: &Context<App>) {
        if let Some(index_from_back) = self.windows.iter().rev().position(|wrapper| wrapper.window.is_closable(state)) {
            let index = self.windows.len() - 1 - index_from_back;
            self.remove_window(index);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_window_with_class(&mut self, window_class: App::WindowClass) {
        if let Some(index_from_back) = self
            .windows
            .iter()
            .rev()
            .map(|wrapper| wrapper.window.get_class())
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
                .get_class()
                .map(|class| !exceptions.contains(&class))
                .unwrap_or(true)
            {
                self.remove_window(index);
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn process_events(&mut self, custom_events: &mut Vec<App::CustomEvent>) {
        for event in self.event_queue.drain() {
            match event {
                // This case should never be hit. FocusElement needs to be converted to
                // FocusElementPost in the event queue while the layout is still alive.
                Event::FocusElement { .. } => {}
                Event::FocusElementPost { element_id } => self.focused_element = Some(element_id),
                Event::Unfocus => self.focused_element = None,
                Event::SetMouseMode { mouse_mode } => self.mouse_mode = mouse_mode,
                Event::Application { custom_event } => custom_events.push(custom_event),
                Event::OpenOverlay {
                    element,
                    position,
                    size,
                    window_id,
                } => {
                    self.overlay_element = Some(OverlayElement {
                        element,
                        store: InternalElementStore::root(&mut self.generator),
                        position,
                        size,
                        window_id,
                    });
                }
                Event::MoveWindowToTop { window_id } => {
                    if let Some(window_index) = self.windows.iter().position(|wrapper| wrapper.data.id == window_id) {
                        let window = self.windows.remove(window_index);
                        self.windows.push(window);
                    }
                }
                Event::CloseWindow { window_id } => {
                    if let Some(index) = self.windows.iter().position(|wrapper| wrapper.data.id == window_id) {
                        self.windows.remove(index);
                    }
                }
                Event::CloseOverlay => {
                    self.overlay_element = None;
                    self.overlay_layout = None;
                }
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn lay_out_windows<'a>(
        &'a mut self,
        state: &'a Context<App>,
        interface_scaling: f32,
        mouse_position: App::Position,
    ) -> InterfaceFrame<'a, App> {
        if let Some(overlay_element) = &mut self.overlay_element {
            match self.windows.iter().find(|wrapper| wrapper.data.id == overlay_element.window_id) {
                Some(wrapper) => {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("create overlay element layout info");

                    let available_area = Area {
                        left: overlay_element.position.left(),
                        top: overlay_element.position.top(),
                        width: overlay_element.size.width(),
                        height: overlay_element.size.height(),
                    };

                    let store = ElementStoreMut::new(&mut overlay_element.store, &mut self.generator, overlay_element.window_id);
                    let mut resolver = Resolver::new(available_area, 0.0, &self.text_layouter);

                    App::set_current_theme_type(wrapper.window.get_theme_type());

                    overlay_element.element.create_layout_info(state, store, &mut resolver);
                }
                // Window was closed while the overlay was open, so we just close it.
                None => self.overlay_element = None,
            }
        }

        // SAFETY:
        //
        // This is safe as long as the `Drop` implementation of the `InterfaceFrame`
        // clears the `Layout`s, removing any references with the lifetime 'a
        // from the struct. If drop does not clear the layout or the clear
        // implementation of drop is incorrect we will end up with dangling
        // references.
        let this = unsafe { std::mem::transmute::<&'a mut Interface<'static, App>, &'a mut Interface<'a, App>>(self) };

        this.windows.iter_mut().for_each(|wrapper| {
            #[cfg(feature = "debug")]
            korangar_debug::profile_block!("create window layout info");

            wrapper.display_information = wrapper.window.create_layout_info(
                state,
                &mut this.window_store,
                &mut wrapper.data,
                &mut this.generator,
                &this.text_layouter,
                this.window_size,
            );
        });

        let mut hovered_window = None;

        if let Some(overlay_element) = &this.overlay_element {
            #[cfg(feature = "debug")]
            korangar_debug::profile_block!("lay out overlay element");

            // SAFETY:
            //
            // Window is guaranteed to exist since we check for that when creating the
            // layout info for the overlay.
            let wrapper = this
                .windows
                .iter()
                .find(|wrapper| wrapper.data.id == overlay_element.window_id)
                .unwrap();

            let position = App::Position::new(
                wrapper.display_information.real_area.left,
                wrapper.display_information.real_area.top,
            );

            let layout = this.overlay_layout.get_or_insert_default();
            layout.update(
                interface_scaling,
                position,
                mouse_position,
                this.focused_element,
                true,
                &this.mouse_mode,
            );

            let store = ElementStore::new(&overlay_element.store, overlay_element.window_id);

            App::set_current_theme_type(wrapper.window.get_theme_type());

            let overlay_area = Area {
                left: overlay_element.position.left(),
                top: overlay_element.position.top(),
                width: overlay_element.size.width(),
                height: overlay_element.size.height(),
            };

            if overlay_area.check().dont_mark().run(layout) {
                hovered_window = Some(overlay_element.window_id);
            }

            overlay_element.element.lay_out(state, store, &(), layout);
        }

        this.windows.iter().rev().for_each(|wrapper| {
            #[cfg(feature = "debug")]
            korangar_debug::profile_block!("lay out window");

            let position = App::Position::new(
                wrapper.display_information.real_area.left,
                wrapper.display_information.real_area.top,
            );

            let layout = this.window_layouts.entry(wrapper.data.id).or_default();
            layout.update(
                interface_scaling,
                position,
                mouse_position,
                this.focused_element,
                hovered_window.is_none(),
                &this.mouse_mode,
            );

            wrapper.window.lay_out(state, &this.window_store, &wrapper.data, layout);

            if hovered_window.is_none() && layout.is_hovered() {
                hovered_window = Some(wrapper.data.id);
            }
        });

        InterfaceFrame {
            windows: &this.windows,
            window_layouts: &mut this.window_layouts,
            overlay_layout: &mut this.overlay_layout,
            event_queue: &mut this.event_queue,
            window_size: this.window_size,
            mouse_mode: &this.mouse_mode,
            hovered_window,
            interface_scaling,
            text_layouter: &this.text_layouter,
        }
    }
}

pub struct InterfaceFrame<'a, App: Application> {
    windows: &'a [WindowWrapper<App>],
    window_layouts: &'a mut BTreeMap<u64, WindowLayout<'a, App>>,
    overlay_layout: &'a mut Option<WindowLayout<'a, App>>,
    event_queue: &'a mut EventQueue<App>,
    mouse_mode: &'a MouseMode<App>,
    window_size: App::Size,
    hovered_window: Option<u64>,
    interface_scaling: f32,
    text_layouter: &'a App::TextLayouter,
}

impl<App: Application> InterfaceFrame<'_, App> {
    pub fn is_interface_hovered(&self) -> bool {
        self.hovered_window.is_some()
    }

    #[cfg(feature = "debug")]
    pub fn render_click_areas(&self, renderer: &App::Renderer, color: App::Color) {
        self.window_layouts
            .values()
            .for_each(|layout| layout.render_click_areas(renderer, color));

        if let Some(overlay_layout) = &self.overlay_layout {
            overlay_layout.render_click_areas(renderer, color);
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_drop_areas(&self, renderer: &App::Renderer, color: App::Color) {
        self.window_layouts
            .values()
            .for_each(|layout| layout.render_drop_areas(renderer, color));

        if let Some(overlay_layout) = &self.overlay_layout {
            overlay_layout.render_drop_areas(renderer, color);
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_scroll_areas(&self, renderer: &App::Renderer, color: App::Color) {
        self.window_layouts
            .values()
            .for_each(|layout| layout.render_scroll_areas(renderer, color));

        if let Some(overlay_layout) = &self.overlay_layout {
            overlay_layout.render_scroll_areas(renderer, color);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render user interface"))]
    pub fn render(
        &mut self,
        state: &Context<App>,
        renderer: &App::Renderer,
        tooltip_theme: &TooltipTheme<App>,
        mouse_position: App::Position,
    ) {
        // TODO: Don't allocate every frame. Move this to the interface as well.
        let mut tooltips = Vec::new();

        self.windows.iter().for_each(|wrapper| {
            let layout = self.window_layouts.get_mut(&wrapper.data.id).unwrap();
            layout.render(renderer, self.text_layouter);
            layout.update_tooltips(&mut tooltips);
        });

        if let Some(layout) = &mut self.overlay_layout {
            layout.render(renderer, self.text_layouter);
            layout.update_tooltips(&mut tooltips);
        }

        if let MouseMode::MovingWindow { window_id } = self.mouse_mode {
            self.render_window_anchors(state, renderer, *window_id);
        }

        if !tooltips.is_empty() {
            self.render_tooltips(renderer, tooltip_theme, &tooltips, mouse_position);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_window_anchors(&self, state: &Context<App>, renderer: &App::Renderer, window_id: u64) {
        if let Some(wrapper) = self.windows.iter().find(|wrapper| wrapper.data.id == window_id) {
            App::set_current_theme_type(wrapper.window.get_theme_type());

            let anchor_color = *state.get(&theme::theme().window().anchor_color());
            let closest_anchor_color = *state.get(&theme::theme().window().closest_anchor_color());

            wrapper.data.anchor.render_screen_anchors(
                renderer,
                anchor_color,
                closest_anchor_color,
                self.window_size,
                self.interface_scaling,
            );

            let window_display_size = App::Size::new(
                wrapper.display_information.real_area.width * self.interface_scaling,
                wrapper.display_information.display_height * self.interface_scaling,
            );

            let position = App::Position::new(
                wrapper.display_information.real_area.left,
                wrapper.display_information.real_area.top,
            );

            wrapper.data.anchor.render_window_anchors(
                renderer,
                anchor_color,
                closest_anchor_color,
                position,
                window_display_size,
                self.interface_scaling,
            );
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_tooltips(
        &self,
        renderer: &App::Renderer,
        tooltip_theme: &TooltipTheme<App>,
        tooltips: &[&str],
        mouse_position: App::Position,
    ) {
        let background_color = tooltip_theme.background_color;
        let foreground_color = tooltip_theme.foreground_color;
        let highlight_color = tooltip_theme.highlight_color;
        let font_size = tooltip_theme.font_size.scaled(self.interface_scaling);
        let corner_diameter = tooltip_theme.corner_diameter.scaled(self.interface_scaling);
        let border = tooltip_theme.border * self.interface_scaling;
        let gap = tooltip_theme.gap * self.interface_scaling;
        let mouse_offset = tooltip_theme.mouse_offset * self.interface_scaling;

        let total_offset = border * 2.0 + mouse_offset;
        let half_window_size = App::Size::new(self.window_size.width() / 2.0, self.window_size.height() / 2.0);
        let available_width = match mouse_position.left() > half_window_size.width() {
            true => mouse_position.left() - total_offset,
            false => self.window_size.width() - mouse_position.left() - total_offset,
        };

        let mut vertical_offset = 0.0;
        let mut forwards_iterator = tooltips.iter();
        let mut backwards_iterator = tooltips.iter().rev();

        let iterator: &mut dyn Iterator<Item = &&str> = match mouse_position.top() > half_window_size.height() {
            true => &mut backwards_iterator,
            false => &mut forwards_iterator,
        };

        for tooltip in iterator {
            let (text_dimensions, font_size) = self.text_layouter.get_text_dimensions(
                tooltip,
                foreground_color,
                highlight_color,
                font_size,
                available_width,
                tooltip_theme.overflow_behavior,
            );

            let tooltip_left = match mouse_position.left() > half_window_size.width() {
                true => mouse_position.left() - text_dimensions.width() - total_offset,
                false => mouse_position.left() + mouse_offset,
            };

            let tooltip_top = match mouse_position.top() > half_window_size.height() {
                true => mouse_position.top() - text_dimensions.height() / 2.0 - border - vertical_offset,
                false => mouse_position.top() - text_dimensions.height() / 2.0 - border + vertical_offset,
            };

            vertical_offset += text_dimensions.height() + border * 2.0 + gap;

            // TODO: Actually get the text dimensions and scale the tooltip size.

            renderer.render_rectangle(
                App::Position::new(tooltip_left, tooltip_top),
                App::Size::new(text_dimensions.width() + border * 2.0, text_dimensions.height() + border * 2.0),
                App::Clip::unbound(),
                corner_diameter,
                background_color,
            );

            renderer.render_text(
                tooltip,
                App::Position::new(tooltip_left + border, tooltip_top + border),
                available_width,
                App::Clip::unbound(),
                foreground_color,
                highlight_color,
                font_size,
            );
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn click(&mut self, state: &Context<App>, click_position: App::Position, mouse_button: MouseButton) {
        self.event_queue.queue(Event::Unfocus);
        self.event_queue.queue(Event::CloseOverlay);

        if let Some(hovered_window) = self.hovered_window {
            self.event_queue.queue(Event::MoveWindowToTop { window_id: hovered_window });
        }

        if let Some(layout) = &self.overlay_layout
            && layout.handle_click(state, self.event_queue, click_position, mouse_button)
        {
            return;
        }

        if let Some(window_id) = &self.hovered_window {
            let layout = self.window_layouts.get(window_id).unwrap();

            layout.handle_click(state, self.event_queue, click_position, mouse_button);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn drop(&mut self, state: &Context<App>, drop_position: App::Position) {
        self.event_queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::Default,
        });

        if let Some(layout) = &self.overlay_layout
            && layout.handle_drop(state, self.event_queue, drop_position, self.mouse_mode)
        {
            return;
        }

        if let Some(window_id) = &self.hovered_window {
            let layout = self.window_layouts.get(window_id).unwrap();

            layout.handle_drop(state, self.event_queue, drop_position, self.mouse_mode);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn scroll(&mut self, state: &Context<App>, mouse_position: App::Position, delta: f32) {
        if let Some(layout) = &self.overlay_layout
            && layout.handle_scroll(state, self.event_queue, mouse_position, delta)
        {
            return;
        }

        if let Some(window_id) = &self.hovered_window {
            let layout = self.window_layouts.get(window_id).unwrap();

            layout.handle_scroll(state, self.event_queue, mouse_position, delta);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn input_characters(&mut self, state: &Context<App>, characters: &[char]) -> bool {
        let mut input_handled = false;

        if let Some(layout) = &self.overlay_layout {
            for character in characters {
                input_handled |= layout.handle_character(state, self.event_queue, *character);
            }
        }

        for wrapper in self.windows {
            let layout = self.window_layouts.get(&wrapper.data.id).unwrap();

            for character in characters {
                input_handled |= layout.handle_character(state, self.event_queue, *character);
            }
        }

        input_handled
    }

    pub fn set_mouse_mode(&mut self, mouse_mode: impl Into<MouseMode<App>>) {
        self.event_queue.queue(Event::SetMouseMode {
            mouse_mode: mouse_mode.into(),
        });
    }

    pub fn focus_element(&mut self, focus_id: impl Any) {
        self.event_queue.queue(Event::FocusElement {
            focus_id: focus_id.focus_id(),
        });
    }

    pub fn unfocus(&mut self) {
        self.event_queue.queue(Event::Unfocus);
    }
}

impl<App: Application> Drop for InterfaceFrame<'_, App> {
    fn drop(&mut self) {
        // Convert FocusElement to FocusElementPost. This needs to be done before the
        // layouts are cleared. If a focus id could not be resolved we leave the event
        // as is, it will be ignored when processing the events.
        self.event_queue.iter_mut().for_each(|event| {
            if let Event::FocusElement { focus_id } = event {
                for layout in self.window_layouts.values() {
                    if let Some(element_id) = layout.try_resolve_focus_id(*focus_id) {
                        *event = Event::FocusElementPost { element_id };
                        return;
                    }
                }
            }
        });

        self.window_layouts.values_mut().for_each(|layout| layout.clear());

        if let Some(layout) = self.overlay_layout {
            layout.clear();
        }
    }
}
