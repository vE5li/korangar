use std::any::Any;
use std::marker::PhantomData;

pub use interface_macros::StateWindow;
use rust_state::{Context, Path, RustState, Selector};
use store::WindowStore;

use crate::application::{Application, CornerRadiusTrait, PositionTrait, SizeTrait};
use crate::element::id::ElementIdGenerator;
use crate::element::{DefaultLayoutInfoSet, ElementSet};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Layout, Resolver};
use crate::theme::{ThemePathGetter, theme};

mod anchor;

pub use anchor::{Anchor, AnchorPoint};

pub mod store {
    use std::cell::UnsafeCell;
    use std::collections::HashMap;

    use crate::element::id::ElementIdGenerator;
    use crate::element::store::ElementStore;

    #[derive(Default)]
    pub struct WindowStore {
        // The element stores need to be in a Box so that we can safely pass out references
        // to them without worrying about relocation of the hashmap when inserting new children.
        stores: HashMap<u64, Box<ElementStore>>,
    }

    impl WindowStore {
        pub fn get_or_create_from_window_id(&mut self, window_id: u64, generator: &mut ElementIdGenerator) -> &mut ElementStore {
            self.stores
                .entry(window_id)
                .or_insert_with(|| Box::new(ElementStore::root(generator)))
        }

        pub fn get_from_window_id(&self, window_id: u64) -> &ElementStore {
            self.stores.get(&window_id).expect("This shouldn't happen")
        }
    }
}

// TODO: Rename
pub trait WindowTrait<App: Application> {
    fn get_window_class(&self) -> Option<App::WindowClass>;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut WindowStore,
        data: &WindowData<App>,
        generator: &mut ElementIdGenerator,
        window_size: App::Size,
    ) -> DisplayInformation<App>;

    fn do_layout<'a>(&'a self, state: &'a Context<App>, store: &'a WindowStore, data: &'a WindowData<App>, layout: &mut Layout<'a, App>);
}

// TODO: Rename this to StateWindow
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
    pub minimum_width: f32,
    pub maximum_width: f32,
    pub minimum_height: f32,
    pub maximum_height: f32,
    pub title_height: f32,
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

pub struct DisplayInformation<App>
where
    App: Application,
{
    pub real_position: App::Position,
    pub real_size: App::Size,
    pub display_height: f32,
}

pub struct WindowLayoutInfoSet<T> {
    area: Area,
    title_area: Area,
    children: T,
}

pub struct Window<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, Elements>
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
    pub font_size: F,
    pub gaps: G,
    pub border: H,
    pub corner_radius: I,
    pub closable: J,
    pub minimum_width: K,
    pub maximum_width: L,
    pub minimum_height: M,
    pub maximum_height: N,
    pub theme: App::ThemeType,
    pub class: Option<App::WindowClass>,
    pub elements: Elements,
    pub layout_info: Option<WindowLayoutInfoSet<<Elements as ElementSet<App>>::LayoutInfo>>,
}

impl<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, Elements> WindowTrait<App>
    for Window<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, Elements>
where
    App: Application,
    Title: AsRef<str>,
    A: Selector<App, Title>,
    B: Selector<App, App::Color>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, f32>,
    F: Selector<App, App::FontSize>,
    G: Selector<App, f32>,
    H: Selector<App, f32>,
    I: Selector<App, App::CornerRadius>,
    J: Selector<App, bool>,
    K: Selector<App, f32>,
    L: Selector<App, f32>,
    M: Selector<App, f32>,
    N: Selector<App, f32>,
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
        data: &WindowData<App>,
        generator: &mut ElementIdGenerator,
        window_size: App::Size,
    ) -> DisplayInformation<App> {
        let store = store.get_or_create_from_window_id(data.id, generator);

        App::set_current_theme_type(self.theme);

        let title_height = *state.get(&self.title_height);

        // Adjust size
        let real_size = {
            let minimum_width = *state.get(&self.minimum_width);
            let maximum_width = *state.get(&self.maximum_width);
            let minimum_height = *state.get(&self.minimum_height);
            let maximum_height = *state.get(&self.maximum_height);

            App::Size::new(
                data.size.width().min(maximum_width).max(minimum_width),
                data.size.height().min(maximum_height).max(minimum_height),
            )
        };

        // Adjust position
        let real_position = {
            let anchor_position = data.anchor.to_position(window_size, real_size);
            let half_width = real_size.width() / 2.0;

            App::Position::new(
                anchor_position.left().max(-half_width).min(window_size.width() - half_width),
                anchor_position.top().max(0.0).min(window_size.height() - title_height),
            )
        };

        let available_area = Area {
            x: real_position.left(),
            y: real_position.top(),
            width: real_size.width(),
            height: real_size.height(),
        };

        let mut resolver = Resolver::new(available_area, 0.0);

        let title_area = resolver.with_height(title_height);

        let (area, children) = resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
            self.elements.create_layout_info(state, store, generator, resolver)
        });

        // FIX: Content needs to respect the max size.
        let area = Area {
            x: title_area.x,
            y: title_area.y,
            width: title_area.width,
            height: title_area.height + area.height,
        };

        self.layout_info = Some(WindowLayoutInfoSet {
            area,
            title_area,
            children,
        });

        let display_height = area.height;

        DisplayInformation {
            real_position,
            real_size,
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
            let close_button_area = Area {
                x: layout_info.title_area.x + layout_info.title_area.width - 40.0,
                y: layout_info.title_area.y,
                width: 30.0,
                height: layout_info.title_area.height,
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

        layout.with_clip_layer(layout_info.area, |layout| {
            layout.with_layer(|layout| {
                self.elements.layout_element(state, store, &layout_info.children, layout);
            });
        });

        let title_color = match layout.is_area_hovered_and_active(layout_info.title_area) {
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

        if layout.is_area_hovered_and_active(layout_info.title_area) {
            layout.add_window_move_area(layout_info.title_area, data.id);
            layout.mark_hovered();
        }

        layout.add_rectangle(
            layout_info.area,
            *state.get(&self.corner_radius),
            *state.get(&self.background_color),
        );

        // FIX: Add height to the check as well. Currently there is no concept of an
        // window with a fixed height and a flexible one, so after that is
        // implemented this can be corrected.
        if *state.get(&self.minimum_width) != *state.get(&self.maximum_width) {
            // TODO: Compute this better.
            let resize_area = Area {
                x: layout_info.area.x + layout_info.area.width - 14.0,
                y: layout_info.area.y + layout_info.area.height - 14.0,
                width: 14.0,
                height: 14.0,
            };

            // TEMP
            layout.add_rectangle(
                resize_area,
                App::CornerRadius::new(0.0, 0.0, state.get(&self.corner_radius).bottom_right(), 0.0),
                *state.get(&self.title_color),
            );

            if let Some((close_button_area, close_button_color)) = close_button {
                layout.add_rectangle(
                    close_button_area,
                    App::CornerRadius::new(0.0, 0.0, 0.0, 0.0),
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

            if layout.is_area_hovered_and_active(resize_area) {
                layout.add_window_resize_area(resize_area, data.id);
                layout.mark_hovered();
            }
        }

        if layout.is_area_hovered_and_active(layout_info.area) {
            layout.mark_hovered();
        }
    }
}
