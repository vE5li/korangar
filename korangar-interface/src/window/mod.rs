mod anchor;
pub mod store;

use std::marker::PhantomData;

pub use anchor::{Anchor, AnchorPoint};
pub use interface_macros::StateWindow;
use rust_state::{Path, RustState, Selector, State};
use store::WindowStore;

use crate::MouseMode;
use crate::application::{Application, CornerDiameter, Position, ShadowPadding, Size};
use crate::element::Element;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::event::{ClickHandler, Event};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{MouseButton, ResizeMode, Resolver, WindowLayout};
use crate::prelude::EventQueue;
use crate::theme::{ThemePathGetter, theme};

mod private {
    /// Sealed trait to avoid outside implementations of
    /// [`Window`](super::Window).
    pub trait Sealed {}
}

/// This trait is only used to abstract over [`WindowInternal`] and it's
/// generics and is not supposed to be implemented outised this crate.
pub trait Window<App: Application>: private::Sealed {
    /// Get the window class of the window (if any).
    fn get_class(&self) -> Option<App::WindowClass>;

    /// Get the window theme.
    fn get_theme_type(&self) -> App::ThemeType;

    /// Returns if the window is closable or not.
    fn is_closable(&self, state: &State<App>) -> bool;

    /// Create the layout info for the window.
    #[allow(private_interfaces)]
    fn create_layout_info(
        &mut self,
        state: &State<App>,
        store: &mut WindowStore,
        data: &mut WindowData<App>,
        generator: &mut ElementIdGenerator,
        text_layouter: &App::TextLayouter,
        window_size: App::Size,
    ) -> DisplayInformation;

    /// Lay out the window.
    #[allow(private_interfaces)]
    fn lay_out<'a>(&'a self, state: &'a State<App>, store: &'a WindowStore, data: &'a WindowData<App>, layout: &mut WindowLayout<'a, App>);
}

/// An application specific custom window.
///
/// Can be passed to [`open_window`](crate::Interface::open_window).
pub trait CustomWindow<App>
where
    App: Application,
{
    /// Return the window class of the window (if any).
    fn window_class() -> Option<App::WindowClass> {
        None
    }

    /// Convert to a real window.
    fn to_window<'a>(self) -> impl Window<App> + 'a;
}

/// A window for inspecting a part of the application state.
///
/// Can be passed to [`open_state_window`](crate::Interface::open_state_window).
///
/// Implementing this trait is generally discouraged, see [`CustomWindow`] for
/// custom windows.
pub trait StateWindow<App>
where
    App: Application,
{
    /// Return the window class of the window (if any).
    fn window_class() -> Option<App::WindowClass> {
        None
    }

    /// Create a new immutable inspector window of the state.
    fn to_window<'a>(self_path: impl Path<App, Self>) -> impl Window<App> + 'a;

    /// Create a new mutable inspector window of the state.
    fn to_window_mut<'a>(self_path: impl Path<App, Self>) -> impl Window<App> + 'a;
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowTheme<App>
where
    App: Application,
{
    pub title_color: App::Color,
    pub hovered_title_color: App::Color,
    pub background_color: App::Color,
    pub highlight_color: App::Color,
    pub shadow_color: App::Color,
    pub shadow_padding: App::ShadowPadding,
    pub gaps: f32,
    pub border: f32,
    pub corner_diameter: App::CornerDiameter,
    pub close_button_size: App::Size,
    pub close_button_corner_diameter: App::CornerDiameter,
    pub minimum_width: f32,
    pub maximum_width: f32,
    pub minimum_height: f32,
    pub maximum_height: f32,
    pub title_height: f32,
    pub title_gap: f32,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
    pub anchor_color: App::Color,
    pub closest_anchor_color: App::Color,
}

pub struct WindowData<App>
where
    App: Application,
{
    pub id: u64,
    pub anchor: Anchor<App>,
    pub size: App::Size,
}

pub(crate) struct DisplayInformation {
    pub real_area: Area,
    pub display_height: f32,
}

pub struct WindowLayoutInfoSet<T> {
    area: Area,
    title_area: Area,
    children: T,
}

struct ResizeClickHandler {
    window_id: u64,
    resize_mode: ResizeMode,
}

impl ResizeClickHandler {
    fn new(resize_mode: ResizeMode) -> Self {
        Self { window_id: 0, resize_mode }
    }

    fn update(&mut self, window_id: u64) {
        self.window_id = window_id;
    }
}

impl<App: Application> ClickHandler<App> for ResizeClickHandler {
    fn handle_click(&self, _: &State<App>, queue: &mut EventQueue<App>) {
        let Self { window_id, resize_mode } = *self;

        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::ResizingWindow { resize_mode, window_id },
        });
    }
}

#[derive(Default)]
struct MoveClickHandler {
    window_id: u64,
}

impl MoveClickHandler {
    fn update(&mut self, window_id: u64) {
        self.window_id = window_id;
    }
}

impl<App: Application> ClickHandler<App> for MoveClickHandler {
    fn handle_click(&self, _: &State<App>, queue: &mut EventQueue<App>) {
        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::MovingWindow { window_id: self.window_id },
        });
    }
}

#[derive(Default)]
struct CloseClickHandler {
    window_id: u64,
}

impl CloseClickHandler {
    fn update(&mut self, window_id: u64) {
        self.window_id = window_id;
    }
}

impl<App: Application> ClickHandler<App> for CloseClickHandler {
    fn handle_click(&self, _: &State<App>, queue: &mut EventQueue<App>) {
        queue.queue(Event::CloseWindow { window_id: self.window_id });
    }
}

pub struct WindowInternal<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
where
    App: Application,
    Elements: Element<App>,
{
    title_marker: PhantomData<Title>,
    title: A,
    title_color: B,
    hovered_title_color: C,
    background_color: D,
    highlight_color: E,
    shadow_color: F,
    shadow_padding: G,
    title_height: H,
    title_gap: I,
    font_size: J,
    gaps: K,
    border: L,
    corner_diameter: M,
    closable: N,
    resizable: O,
    close_button_size: P,
    close_button_corner_diameter: Q,
    minimum_width: R,
    maximum_width: S,
    minimum_height: T,
    maximum_height: U,
    theme: App::ThemeType,
    class: Option<App::WindowClass>,
    elements: Elements,
    // HACK: This is a bit ugly since all of these store the window_id. Ideally, we would be able
    // to inject some data into the click action instead.
    close_click_action: CloseClickHandler,
    move_click_action: MoveClickHandler,
    horizontal_resize_click_action: ResizeClickHandler,
    vertical_resize_click_action: ResizeClickHandler,
    resize_click_action: ResizeClickHandler,
    layout_info: Option<WindowLayoutInfoSet<<Elements as Element<App>>::LayoutInfo>>,
}

impl<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
    WindowInternal<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
where
    App: Application,
    Elements: Element<App>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        title: A,
        title_color: B,
        hovered_title_color: C,
        background_color: D,
        highlight_color: E,
        shadow_color: F,
        shadow_padding: G,
        title_height: H,
        title_gap: I,
        font_size: J,
        gaps: K,
        border: L,
        corner_diameter: M,
        closable: N,
        resizable: O,
        close_button_size: P,
        close_button_corner_diameter: Q,
        minimum_width: R,
        maximum_width: S,
        minimum_height: T,
        maximum_height: U,
        theme: App::ThemeType,
        class: Option<App::WindowClass>,
        elements: Elements,
    ) -> Self {
        Self {
            title_marker: PhantomData,
            title,
            title_color,
            hovered_title_color,
            background_color,
            highlight_color,
            shadow_color,
            shadow_padding,
            title_height,
            title_gap,
            font_size,
            gaps,
            border,
            corner_diameter,
            closable,
            resizable,
            close_button_size,
            close_button_corner_diameter,
            minimum_width,
            maximum_width,
            minimum_height,
            maximum_height,
            theme,
            class,
            elements,
            close_click_action: CloseClickHandler::default(),
            move_click_action: MoveClickHandler::default(),
            horizontal_resize_click_action: ResizeClickHandler::new(ResizeMode::Horizontal),
            vertical_resize_click_action: ResizeClickHandler::new(ResizeMode::Vertical),
            resize_click_action: ResizeClickHandler::new(ResizeMode::Both),
            layout_info: None,
        }
    }
}

impl<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements> private::Sealed
    for WindowInternal<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
where
    App: Application,
    Elements: Element<App>,
{
}

impl<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements> Window<App>
    for WindowInternal<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
where
    App: Application,
    Title: AsRef<str>,
    A: Selector<App, Title>,
    B: Selector<App, App::Color>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::ShadowPadding>,
    H: Selector<App, f32>,
    I: Selector<App, f32>,
    J: Selector<App, App::FontSize>,
    K: Selector<App, f32>,
    L: Selector<App, f32>,
    M: Selector<App, App::CornerDiameter>,
    N: Selector<App, bool>,
    O: Selector<App, bool>,
    P: Selector<App, App::Size>,
    Q: Selector<App, App::CornerDiameter>,
    R: Selector<App, f32>,
    S: Selector<App, f32>,
    T: Selector<App, f32>,
    U: Selector<App, f32>,
    Elements: Element<App>,
    <Elements as Element<App>>::LayoutInfo: 'static,
{
    fn get_class(&self) -> Option<App::WindowClass> {
        self.class
    }

    fn get_theme_type(&self) -> <App as Application>::ThemeType {
        self.theme
    }

    fn is_closable(&self, state: &State<App>) -> bool {
        *state.get(&self.closable)
    }

    #[allow(private_interfaces)]
    fn create_layout_info(
        &mut self,
        state: &State<App>,
        store: &mut WindowStore,
        data: &mut WindowData<App>,
        generator: &mut ElementIdGenerator,
        text_layouter: &App::TextLayouter,
        window_size: App::Size,
    ) -> DisplayInformation {
        self.close_click_action.update(data.id);
        self.move_click_action.update(data.id);
        self.horizontal_resize_click_action.update(data.id);
        self.vertical_resize_click_action.update(data.id);
        self.resize_click_action.update(data.id);

        let store = store.get_or_create_from_window_id(data.id, generator);

        App::set_current_theme_type(self.theme);

        let title_height = *state.get(&self.title_height);
        let title_gap = *state.get(&self.title_gap);

        let minimum_width = *state.get(&self.minimum_width);
        let maximum_width = *state.get(&self.maximum_width);
        let minimum_height = *state.get(&self.minimum_height);
        let maximum_height = *state.get(&self.maximum_height);

        let adjusted_size = App::Size::new(
            data.size.width().min(maximum_width).max(minimum_width),
            data.size.height().min(maximum_height).max(minimum_height),
        );

        if data.anchor.is_initializing() {
            data.anchor.initialize(window_size, adjusted_size);
        }

        // Adjust position
        let real_position = {
            let anchor_position = data.anchor.to_position(window_size);
            let half_width = adjusted_size.width() / 2.0;

            App::Position::new(
                anchor_position.left().max(-half_width).min(window_size.width() - half_width),
                anchor_position.top().max(0.0).min(window_size.height() - title_height),
            )
        };

        let available_area = Area {
            left: real_position.left(),
            top: real_position.top(),
            width: adjusted_size.width(),
            height: adjusted_size.height(),
        };

        let mut resolver = Resolver::new(available_area, 0.0, text_layouter);

        let title_area = resolver.with_height(title_height);

        let (area, children) = resolver.with_derived_borderless(*state.get(&self.gaps), *state.get(&self.border), title_gap, |resolver| {
            self.elements.create_layout_info(state, store, resolver)
        });

        let area = Area {
            left: title_area.left,
            top: title_area.top,
            width: title_area.width,
            height: (title_area.height + area.height).min(maximum_height).max(minimum_height),
        };

        self.layout_info = Some(WindowLayoutInfoSet {
            area,
            title_area,
            children,
        });

        let display_height = area.height;

        DisplayInformation {
            real_area: area,
            display_height,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    #[allow(private_interfaces)]
    fn lay_out<'a>(&'a self, state: &'a State<App>, store: &'a WindowStore, data: &'a WindowData<App>, layout: &mut WindowLayout<'a, App>) {
        let store = store.get_from_window_id(data.id);
        let layout_info = self.layout_info.as_ref().expect("no layout present");

        App::set_current_theme_type(self.theme);

        if layout_info.area.check().dont_mark().run(layout) {
            layout.set_hovered();
        }

        let close_button = if *state.get(&self.closable) {
            let close_button_size = *state.get(&self.close_button_size);
            let offset = layout_info.title_area.height - close_button_size.height();

            let close_button_area = Area {
                left: layout_info.title_area.left + layout_info.title_area.width - close_button_size.width() - *state.get(&self.border),
                top: layout_info.title_area.top + offset / 2.0,
                width: close_button_size.width(),
                height: layout_info.title_area.height - offset,
            };

            let close_button_color = match close_button_area.check().run(layout) {
                true => {
                    layout.register_click_handler(MouseButton::Left, &self.close_click_action);

                    *state.get(&self.hovered_title_color)
                }
                false => *state.get(&self.title_color),
            };

            Some((close_button_area, close_button_color))
        } else {
            None
        };

        let is_title_hovered = layout_info.title_area.check().run(layout);

        if is_title_hovered {
            layout.register_click_handler(MouseButton::Left, &self.move_click_action);
        }

        let corner_diameter = *state.get(&self.corner_diameter);

        let horizontal_resize_area = Area {
            left: layout_info.area.left + layout_info.area.width - 3.0,
            top: layout_info.area.top + 20.0,
            width: 6.0,
            height: layout_info.area.height - 40.0,
        };
        let vertical_resize_area = Area {
            left: layout_info.area.left + 20.0,
            top: layout_info.area.top + layout_info.area.height - 3.0,
            width: layout_info.area.width - 40.0,
            height: 6.0,
        };

        let radius = corner_diameter.bottom_right() / 2.0;
        let corner_offset = radius - (radius.powi(2) / 2.0).sqrt();
        let resize_area = Area {
            left: layout_info.area.left + layout_info.area.width - corner_offset - 6.0,
            top: layout_info.area.top + layout_info.area.height - corner_offset - 6.0,
            width: 12.0,
            height: 12.0,
        };
        let horizontal_resize_hovered = horizontal_resize_area.check().run(layout);
        let vertical_resize_hovered = vertical_resize_area.check().run(layout);
        let resize_hovered = resize_area.check().run(layout);

        let horizontal_resize_available = *state.get(&self.minimum_width) != *state.get(&self.maximum_width);
        let vertical_resize_availabe = *state.get(&self.resizable);

        if horizontal_resize_hovered && horizontal_resize_available {
            layout.register_click_handler(MouseButton::Left, &self.horizontal_resize_click_action);
            layout.set_hovered();
        } else if vertical_resize_hovered && vertical_resize_availabe {
            layout.register_click_handler(MouseButton::Left, &self.vertical_resize_click_action);
            layout.set_hovered();
        } else if resize_hovered && horizontal_resize_available && vertical_resize_availabe {
            layout.register_click_handler(MouseButton::Left, &self.resize_click_action);
            layout.set_hovered();
        }

        layout.with_clip(layout_info.area, |layout| {
            layout.with_layer(|layout| {
                self.elements
                    .lay_out(state, ElementStore::new(store, data.id), &layout_info.children, layout);
            });
        });

        let title_color = match is_title_hovered {
            true => *state.get(&self.hovered_title_color),
            false => *state.get(&self.title_color),
        };

        layout.add_text(
            layout_info.title_area,
            state.get(&self.title).as_ref(),
            *state.get(&self.font_size),
            title_color,
            *state.get(&self.highlight_color),
            // TODO: Make this configurable in the window.
            *state.get(&theme().window().horizontal_alignment()),
            *state.get(&theme().window().vertical_alignment()),
            *state.get(&theme().window().overflow_behavior()),
        );

        layout.add_rectangle(
            layout_info.area,
            *state.get(&self.corner_diameter),
            *state.get(&self.background_color),
            *state.get(&self.shadow_color),
            *state.get(&self.shadow_padding),
        );

        if horizontal_resize_hovered && horizontal_resize_available
            || matches!(layout.get_mouse_mode(), MouseMode::ResizingWindow {
                resize_mode: ResizeMode::Horizontal,
                window_id,
            } if *window_id == data.id)
        {
            layout.add_rectangle(
                horizontal_resize_area,
                App::CornerDiameter::new(6.0, 6.0, 6.0, 6.0),
                *state.get(&theme().window().closest_anchor_color()),
                // TODO: Properly theme
                *state.get(&theme().window().closest_anchor_color()),
                App::ShadowPadding::none(),
            );
        } else if vertical_resize_hovered && vertical_resize_availabe
            || matches!(layout.get_mouse_mode(), MouseMode::ResizingWindow {
                resize_mode: ResizeMode::Vertical,
                window_id,
            } if *window_id == data.id)
        {
            layout.add_rectangle(
                vertical_resize_area,
                App::CornerDiameter::new(6.0, 6.0, 6.0, 6.0),
                *state.get(&theme().window().closest_anchor_color()),
                // TODO: Properly theme
                *state.get(&theme().window().closest_anchor_color()),
                App::ShadowPadding::none(),
            );
        } else if resize_hovered && horizontal_resize_available && vertical_resize_availabe
            || matches!(layout.get_mouse_mode(), MouseMode::ResizingWindow {
                resize_mode: ResizeMode::Both,
                window_id,
            } if *window_id == data.id)
        {
            layout.add_rectangle(
                resize_area,
                App::CornerDiameter::new(12.0, 12.0, 12.0, 12.0),
                *state.get(&theme().window().closest_anchor_color()),
                // TODO: Properly theme
                *state.get(&theme().window().closest_anchor_color()),
                App::ShadowPadding::none(),
            );
        }

        if let Some((close_button_area, close_button_color)) = close_button {
            layout.add_rectangle(
                close_button_area,
                *state.get(&self.close_button_corner_diameter),
                close_button_color,
                // TODO: Properly theme
                close_button_color,
                App::ShadowPadding::none(),
            );

            // TODO: Use own values
            layout.add_text(
                close_button_area,
                "X",
                *state.get(&self.font_size),
                *state.get(&self.background_color),
                *state.get(&self.highlight_color),
                HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                // TODO: This shouldn't matter at all.
                *state.get(&theme().window().overflow_behavior()),
            );
        }
    }
}
