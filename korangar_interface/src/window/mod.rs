use std::marker::PhantomData;

pub use interface_macros::PrototypeWindow;
use rust_state::{Context, RustState, Selector};
use store::WindowStore;

use crate::application::{Appli, CornerRadiusTrait, PositionTrait, SizeTrait};
use crate::element::ElementSet;
use crate::element::id::ElementIdGenerator;
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
        stores: UnsafeCell<HashMap<u64, Box<ElementStore>>>,
    }

    impl WindowStore {
        pub fn get_from_window_id(&self, window_id: u64, generator: &mut ElementIdGenerator) -> &ElementStore {
            let stores = unsafe { &mut *self.stores.get() };

            stores.entry(window_id).or_insert_with(|| Box::new(ElementStore::root(generator)))
        }
    }
}

// TODO: Rename
pub trait WindowTrait<App: Appli> {
    fn get_window_class(&self) -> Option<App::WindowClass>;

    fn do_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a WindowStore,
        data: &'a WindowData<App>,
        generator: &mut ElementIdGenerator,
        layout: &mut Layout<'a, App>,
    );
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

pub struct Window<App, Title, A, B, C, D, E, F, G, H, I, Elements>
where
    App: Appli,
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
    pub theme: App::ThemeType,
    pub class: Option<App::WindowClass>,
    pub elements: Elements,
}

impl<App, Title, A, B, C, D, E, F, G, H, I, Elements> WindowTrait<App> for Window<App, Title, A, B, C, D, E, F, G, H, I, Elements>
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
    Elements: ElementSet<App>,
{
    fn get_window_class(&self) -> Option<App::WindowClass> {
        self.class
    }

    // TODO: Rename
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn do_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a WindowStore,
        data: &'a WindowData<App>,
        generator: &mut ElementIdGenerator,
        layout: &mut Layout<'a, App>,
    ) {
        let store = store.get_from_window_id(data.id, generator);

        let available_area = Area {
            x: data.position.left(),
            y: data.position.top(),
            width: data.size.width(),
            height: data.size.height(),
        };

        App::set_current_theme_type(self.theme);

        let mut resolver = Resolver::new(available_area, 0.0);

        let mut title_area = Area {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };

        let area = layout.with_clip_layer(|layout| {
            resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
                let title_height = *state.get(&self.title_height);
                title_area = resolver.with_height(title_height);

                // TODO: Very much temp
                layout.push_layer();

                self.elements.create_layout(state, store, generator, resolver, layout);

                // TODO: Very much temp
                layout.pop_layer();
            })
        });

        {
            let title_bottom = title_area.y + title_area.height;

            title_area.x = area.x;
            title_area.width = area.width;
            title_area.y = area.y;
            title_area.height = title_bottom - area.y;
        }

        let title_color = match layout.is_area_hovered_and_active(title_area) {
            true => *state.get(&self.hovered_title_color),
            false => *state.get(&self.title_color),
        };

        layout.add_text(
            title_area,
            state.get(&self.title).as_ref(),
            *state.get(&self.font_size),
            title_color,
            *state.get(&theme().window().text_alignment()),
            *state.get(&theme().window().vertical_alignment()),
        );

        if layout.is_area_hovered_and_active(title_area) {
            layout.add_window_move_area(title_area, data.id);
            layout.mark_hovered();
        }

        layout.add_rectangle(area, *state.get(&self.corner_radius), *state.get(&self.background_color));

        // TODO: Compute this better.
        let resize_area = Area {
            x: area.x + area.width - 14.0,
            y: area.y + area.height - 14.0,
            width: 14.0,
            height: 14.0,
        };

        // TEMP
        layout.add_rectangle(
            resize_area,
            App::CornerRadius::new(0.0, 0.0, state.get(&self.corner_radius).bottom_right(), 0.0),
            *state.get(&self.title_color),
        );

        if layout.is_area_hovered_and_active(resize_area) {
            layout.add_window_resize_area(resize_area, data.id);
            layout.mark_hovered();
        }

        if layout.is_area_hovered_and_active(area) {
            layout.mark_hovered();
        }
    }
}
