use std::any::Any;
use std::marker::PhantomData;

pub use interface_macros::PrototypeWindow;
use rust_state::{Context, RustState, Selector};
use store::WindowStore;

use crate::application::{Appli, CornerRadiusTrait, PositionTrait, SizeTrait};
use crate::element::id::ElementIdGenerator;
use crate::element::{DefaultLayoutedSet, ElementSet};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Layout, Resolver};
use crate::theme::{ThemePathGetter, theme};

mod anchor;
mod prototype;

pub use anchor::{Anchor, AnchorPoint};
pub use prototype::{CustomWindow, PrototypeWindow};

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
pub trait WindowTrait<App: Appli> {
    fn get_window_class(&self) -> Option<App::WindowClass>;

    fn make_layout(
        &mut self,
        state: &Context<App>,
        store: &mut WindowStore,
        data: &mut WindowData<App>,
        generator: &mut ElementIdGenerator,
    );

    fn do_layout<'a>(&'a self, state: &'a Context<App>, store: &'a WindowStore, data: &'a WindowData<App>, layout: &mut Layout<'a, App>);
}

#[derive(RustState)]
pub struct WindowTheme<App>
where
    App: Appli,
{
    pub title_color: App::Color,
    pub hovered_title_color: App::Color,
    pub background_color: App::Color,
    pub gaps: f32,
    pub border: f32,
    pub corner_radius: App::CornerRadius,
    pub title_height: f32,
    pub font_size: App::FontSize,
    pub text_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub anchor_color: App::Color,
    pub closest_anchor_color: App::Color,
}

pub struct WindowData<App>
where
    App: Appli,
{
    pub id: u64,
    pub position: App::Position,
    pub size: App::Size,
}

pub struct WindowLayoutedSet<T> {
    area: Area,
    title_area: Area,
    children: T,
}

pub struct Window<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, Elements>
where
    App: Appli,
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
    pub maximum_height: M,
    pub theme: App::ThemeType,
    pub class: Option<App::WindowClass>,
    pub elements: Elements,
    pub layouted: Option<WindowLayoutedSet<<Elements as ElementSet<App>>::Layouted>>,
}

impl<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, Elements> WindowTrait<App>
    for Window<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, Elements>
where
    App: Appli,
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
    Elements: ElementSet<App>,
    <Elements as ElementSet<App>>::Layouted: 'static,
{
    fn get_window_class(&self) -> Option<App::WindowClass> {
        self.class
    }

    fn make_layout(
        &mut self,
        state: &Context<App>,
        store: &mut WindowStore,
        data: &mut WindowData<App>,
        generator: &mut ElementIdGenerator,
    ) {
        let store = store.get_or_create_from_window_id(data.id, generator);

        {
            let minimum_width = *state.get(&self.minimum_width);
            let maximum_width = *state.get(&self.maximum_width);
            let maximum_height = *state.get(&self.maximum_height);

            data.size = App::Size::new(
                data.size.width().max(minimum_width).min(maximum_width),
                data.size.height().min(maximum_height),
            );
        }

        let available_area = Area {
            x: data.position.left(),
            y: data.position.top(),
            width: data.size.width(),
            height: data.size.height(),
        };

        App::set_current_theme_type(self.theme);

        let mut resolver = Resolver::new(available_area, 0.0);

        let title_height = *state.get(&self.title_height);
        let title_area = resolver.with_height(title_height);

        let (area, children) = resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
            self.elements.make_layout(state, store, generator, resolver)
        });

        let area = Area {
            x: title_area.x,
            y: title_area.y,
            width: title_area.width,
            height: title_area.height + area.height,
        };

        self.layouted = Some(WindowLayoutedSet {
            area,
            title_area,
            children,
        });
    }

    // TODO: Rename
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn do_layout<'a>(&'a self, state: &'a Context<App>, store: &'a WindowStore, data: &'a WindowData<App>, layout: &mut Layout<'a, App>) {
        let store = store.get_from_window_id(data.id);
        let layouted = self.layouted.as_ref().expect("no layout present");

        App::set_current_theme_type(self.theme);

        let close_button = if *state.get(&self.closable) {
            let close_button_area = Area {
                x: layouted.title_area.x + layouted.title_area.width - 40.0,
                y: layouted.title_area.y,
                width: 30.0,
                height: layouted.title_area.height,
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

        layout.with_clip_layer(layouted.area, |layout| {
            layout.push_layer();

            self.elements.create_layout(state, store, &layouted.children, layout);

            layout.pop_layer();
        });

        let title_color = match layout.is_area_hovered_and_active(layouted.title_area) {
            true => *state.get(&self.hovered_title_color),
            false => *state.get(&self.title_color),
        };

        layout.add_text(
            layouted.title_area,
            state.get(&self.title).as_ref(),
            *state.get(&self.font_size),
            title_color,
            *state.get(&theme().window().text_alignment()),
            *state.get(&theme().window().vertical_alignment()),
        );

        if layout.is_area_hovered_and_active(layouted.title_area) {
            layout.add_window_move_area(layouted.title_area, data.id);
            layout.mark_hovered();
        }

        layout.add_rectangle(
            layouted.area,
            *state.get(&self.corner_radius),
            *state.get(&self.background_color),
        );

        // FIX: Add height to the check as well. Currently there is no concept of an
        // window with a fixed height and a flexible one, so after that is
        // implemented this can be corrected.
        if *state.get(&self.minimum_width) != *state.get(&self.maximum_width) {
            // TODO: Compute this better.
            let resize_area = Area {
                x: layouted.area.x + layouted.area.width - 14.0,
                y: layouted.area.y + layouted.area.height - 14.0,
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

        if layout.is_area_hovered_and_active(layouted.area) {
            layout.mark_hovered();
        }
    }
}
