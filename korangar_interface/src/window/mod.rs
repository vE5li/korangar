use std::marker::PhantomData;

pub use interface_macros::PrototypeWindow;
use rust_state::{Context, RustState, Selector};
use store::WindowStore;

use crate::application::{Appli, PositionTrait};
use crate::element::ElementSet;
use crate::element::id::ElementIdGenerator;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Layout, Resolver};
use crate::theme::{ThemePathGetter, theme};

mod anchor;
mod cache;
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
    fn get_window_class(&self) -> Option<&str>;

    fn get_layout(&self) -> (Anchor<App>, App::Size);

    fn offset(&self, available_space: App::Size, offset: App::Position) -> Option<(&str, Anchor<App>)>;

    fn resize(&self, available_space: App::Size, growth: App::Size) -> (Option<&str>, App::Size);

    fn do_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a WindowStore,
        generator: &mut ElementIdGenerator,
        layout: &mut Layout<'a, App>,
        // TODO: Temp
        position: App::Position,
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
    pub window_id: u64,
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
    fn get_window_class(&self) -> Option<&str> {
        // TODO: Return correct window class.
        None
    }

    // TODO: Rename
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn do_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a WindowStore,
        generator: &mut ElementIdGenerator,
        layout: &mut Layout<'a, App>,
        // TODO: Temp
        position: App::Position,
    ) {
        let store = store.get_from_window_id(self.window_id, generator);

        let available_area = Area {
            x: position.left(),
            y: position.top(),
            width: 500.0,
            height: 530.0,
        };

        App::set_current_theme_type(self.theme);

        let mut resolver = Resolver::new(available_area, 0.0);

        let area = layout.with_clip_layer(|layout| {
            resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
                let title_height = *state.get(&self.title_height);
                let title_area = resolver.with_height(title_height);

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

                // TODO: Very much temp
                layout.push_layer();

                self.elements.create_layout(state, store, generator, resolver, layout);

                // TODO: Very much temp
                layout.pop_layer();
            })
        });

        layout.add_rectangle(area, *state.get(&self.corner_radius), *state.get(&self.background_color));

        if layout.is_area_hovered_and_active(area) {
            layout.mark_hovered();
        }
    }

    fn get_layout(&self) -> (Anchor<App>, App::Size) {
        todo!()
    }

    fn offset(&self, available_space: App::Size, offset: App::Position) -> Option<(&str, Anchor<App>)> {
        todo!()
    }

    fn resize(&self, available_space: App::Size, growth: App::Size) -> (Option<&str>, App::Size) {
        todo!()
    }
}
