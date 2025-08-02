mod anchor;
pub mod store;

use std::marker::PhantomData;

pub use anchor::{Anchor, AnchorPoint};
pub use interface_macros::StateWindow;
use rust_state::{Context, Path, RustState, Selector};
use store::WindowStore;

use crate::MouseMode;
use crate::application::{Application, CornerRadiusTrait, PositionTrait, SizeTrait};
use crate::element::ElementSet;
use crate::element::id::ElementIdGenerator;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Layout, ResizeMode, Resolver};
use crate::theme::{ThemePathGetter, theme};

// TODO: Rename
pub trait WindowTrait<App: Application> {
    fn get_window_class(&self) -> Option<App::WindowClass>;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut WindowStore,
        data: &mut WindowData<App>,
        generator: &mut ElementIdGenerator,
        window_size: App::Size,
    ) -> DisplayInformation;

    fn do_layout<'a>(&'a self, state: &'a Context<App>, store: &'a WindowStore, data: &'a WindowData<App>, layout: &mut Layout<'a, App>);
}

pub trait StateWindow<App>
where
    App: Application,
{
    fn window_class() -> Option<App::WindowClass> {
        None
    }

    fn to_window<'a>(self_path: impl Path<App, Self>) -> impl WindowTrait<App> + 'a;

    fn to_window_mut<'a>(self_path: impl Path<App, Self>) -> impl WindowTrait<App> + 'a;
}

pub trait CustomWindow<App>
where
    App: Application,
{
    fn window_class() -> Option<App::WindowClass> {
        None
    }

    fn to_window<'a>(self) -> impl WindowTrait<App> + 'a;
}

#[derive(RustState)]
pub struct WindowTheme<App>
where
    App: Application,
{
    pub title_color: App::Color,
    pub hovered_title_color: App::Color,
    pub background_color: App::Color,
    pub gaps: f32,
    pub border: f32,
    pub corner_radius: App::CornerRadius,
    pub close_button_size: App::Size,
    pub close_button_corner_radius: App::CornerRadius,
    pub minimum_width: f32,
    pub maximum_width: f32,
    pub minimum_height: f32,
    pub maximum_height: f32,
    pub title_height: f32,
    pub title_gap: f32,
    pub font_size: App::FontSize,
    pub text_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
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

pub struct DisplayInformation {
    pub real_area: Area,
    pub display_height: f32,
}

pub struct WindowLayoutInfoSet<T> {
    area: Area,
    title_area: Area,
    children: T,
}

pub struct Window<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Elements>
where
    App: Application,
    Elements: ElementSet<App>,
{
    pub title_marker: PhantomData<Title>,
    pub title: A,
    pub title_color: B,
    pub hovered_title_color: C,
    pub background_color: D,
    pub title_height: E,
    pub title_gap: F,
    pub font_size: G,
    pub gaps: H,
    pub border: I,
    pub corner_radius: J,
    pub closable: K,
    pub resizable: L,
    pub close_button_size: M,
    pub close_button_corner_radius: N,
    pub minimum_width: O,
    pub maximum_width: P,
    pub minimum_height: Q,
    pub maximum_height: R,
    pub theme: App::ThemeType,
    pub class: Option<App::WindowClass>,
    pub elements: Elements,
    pub layout_info: Option<WindowLayoutInfoSet<<Elements as ElementSet<App>>::LayoutInfo>>,
}

impl<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Elements> WindowTrait<App>
    for Window<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Elements>
where
    App: Application,
    Title: AsRef<str>,
    A: Selector<App, Title>,
    B: Selector<App, App::Color>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, f32>,
    F: Selector<App, f32>,
    G: Selector<App, App::FontSize>,
    H: Selector<App, f32>,
    I: Selector<App, f32>,
    J: Selector<App, App::CornerRadius>,
    K: Selector<App, bool>,
    L: Selector<App, bool>,
    M: Selector<App, App::Size>,
    N: Selector<App, App::CornerRadius>,
    O: Selector<App, f32>,
    P: Selector<App, f32>,
    Q: Selector<App, f32>,
    R: Selector<App, f32>,
    Elements: ElementSet<App>,
    <Elements as ElementSet<App>>::LayoutInfo: 'static,
{
    fn get_window_class(&self) -> Option<App::WindowClass> {
        self.class
    }

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut WindowStore,
        data: &mut WindowData<App>,
        generator: &mut ElementIdGenerator,
        window_size: App::Size,
    ) -> DisplayInformation {
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

        let mut resolver = Resolver::new(available_area, 0.0);

        let title_area = resolver.with_height(title_height);

        let (area, children) = resolver.with_derived_borderless(*state.get(&self.gaps), *state.get(&self.border), title_gap, |resolver| {
            self.elements.create_layout_info(state, store, generator, resolver)
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

    // TODO: Rename
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn do_layout<'a>(&'a self, state: &'a Context<App>, store: &'a WindowStore, data: &'a WindowData<App>, layout: &mut Layout<'a, App>) {
        let store = store.get_from_window_id(data.id);
        let layout_info = self.layout_info.as_ref().expect("no layout present");

        App::set_current_theme_type(self.theme);

        let close_button = if *state.get(&self.closable) {
            let close_button_size = *state.get(&self.close_button_size);
            let offset = layout_info.title_area.height - close_button_size.height();

            let close_button_area = Area {
                left: layout_info.title_area.left + layout_info.title_area.width - close_button_size.width() - *state.get(&self.border),
                top: layout_info.title_area.top + offset / 2.0,
                width: close_button_size.width(),
                height: layout_info.title_area.height - offset,
            };

            let close_button_color = match layout.is_area_hovered_and_active(close_button_area) {
                true => {
                    layout.add_window_close_area(close_button_area, data.id);
                    layout.mark_hovered();

                    *state.get(&self.hovered_title_color)
                }
                false => *state.get(&self.title_color),
            };

            Some((close_button_area, close_button_color))
        } else {
            None
        };

        let is_title_hovered = layout.is_area_hovered_and_active(layout_info.title_area);

        if is_title_hovered {
            layout.add_window_move_area(layout_info.title_area, data.id);
            layout.mark_hovered();
        }

        let corner_radius = *state.get(&self.corner_radius);

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

        let radius = corner_radius.bottom_right() / 2.0;
        let corner_offset = radius - (radius.powi(2) / 2.0).sqrt();
        let resize_area = Area {
            left: layout_info.area.left + layout_info.area.width - corner_offset - 6.0,
            top: layout_info.area.top + layout_info.area.height - corner_offset - 6.0,
            width: 12.0,
            height: 12.0,
        };
        let horizontal_resize_hovered = layout.is_area_hovered_and_active(horizontal_resize_area);
        let vertical_resize_hovered = layout.is_area_hovered_and_active(vertical_resize_area);
        let resize_hovered = layout.is_area_hovered_and_active(resize_area);

        let horizontal_resize_available = *state.get(&self.minimum_width) != *state.get(&self.maximum_width);
        let vertical_resize_availabe = *state.get(&self.resizable);

        if horizontal_resize_hovered && horizontal_resize_available {
            layout.add_window_resize_area(horizontal_resize_area, ResizeMode::Horizontal, data.id);
            layout.mark_hovered();
        } else if vertical_resize_hovered && vertical_resize_availabe {
            layout.add_window_resize_area(vertical_resize_area, ResizeMode::Vertical, data.id);
            layout.mark_hovered();
        } else if resize_hovered && horizontal_resize_available && vertical_resize_availabe {
            layout.add_window_resize_area(resize_area, ResizeMode::Both, data.id);
            layout.mark_hovered();
        }

        layout.with_clip_layer(layout_info.area, |layout| {
            layout.with_layer(|layout| {
                self.elements.layout_element(state, store, &layout_info.children, layout);
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
            *state.get(&theme().window().text_alignment()),
            *state.get(&theme().window().vertical_alignment()),
        );

        layout.add_rectangle(
            layout_info.area,
            *state.get(&self.corner_radius),
            *state.get(&self.background_color),
        );

        if horizontal_resize_hovered && horizontal_resize_available
            || matches!(layout.get_mouse_mode(), MouseMode::ResizingWindow {
                resize_mode: ResizeMode::Horizontal,
                window_id,
            } if *window_id == data.id)
        {
            layout.add_rectangle(
                horizontal_resize_area,
                App::CornerRadius::new(6.0, 6.0, 6.0, 6.0),
                *state.get(&theme().window().closest_anchor_color()),
            );
        } else if vertical_resize_hovered && vertical_resize_availabe
            || matches!(layout.get_mouse_mode(), MouseMode::ResizingWindow {
                resize_mode: ResizeMode::Vertical,
                window_id,
            } if *window_id == data.id)
        {
            layout.add_rectangle(
                vertical_resize_area,
                App::CornerRadius::new(6.0, 6.0, 6.0, 6.0),
                *state.get(&theme().window().closest_anchor_color()),
            );
        } else if resize_hovered && horizontal_resize_available && vertical_resize_availabe
            || matches!(layout.get_mouse_mode(), MouseMode::ResizingWindow {
                resize_mode: ResizeMode::Both,
                window_id,
            } if *window_id == data.id)
        {
            layout.add_rectangle(
                resize_area,
                App::CornerRadius::new(12.0, 12.0, 12.0, 12.0),
                *state.get(&theme().window().closest_anchor_color()),
            );
        }

        if let Some((close_button_area, close_button_color)) = close_button {
            layout.add_rectangle(
                close_button_area,
                *state.get(&self.close_button_corner_radius),
                close_button_color,
            );

            // TODO: Use own values
            layout.add_text(
                close_button_area,
                "X",
                *state.get(&self.font_size),
                *state.get(&self.background_color),
                HorizontalAlignment::Center { offset: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
            );
        }

        if layout.is_area_hovered_and_active(layout_info.area) {
            layout.mark_hovered();
        }
    }
}
