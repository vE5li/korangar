use std::cell::RefCell;
use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::Appli;
use crate::element::id::ElementIdGenerator;
use crate::element::store::{ElementStore, Persistent, PersistentData, PersistentExt};
use crate::element::{Element, ElementSet};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Layout, Resolver};
use crate::theme::{ThemePathGetter, theme};

#[derive(RustState)]
pub struct CollapsableTheme<App>
where
    App: Appli,
{
    pub foreground_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub background_color: App::Color,
    pub gaps: f32,
    pub border: f32,
    pub corner_radius: App::CornerRadius,
    pub title_height: f32,
    pub font_size: App::FontSize,
    pub text_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
}

pub struct CollapsableData {
    expanded: RefCell<bool>,
}

impl PersistentData for CollapsableData {
    type Inputs = bool;

    fn new(inputs: Self::Inputs) -> Self {
        Self {
            expanded: RefCell::new(inputs),
        }
    }
}

pub struct Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, Children> {
    pub text_marker: PhantomData<Text>,
    pub text: A,
    pub foreground_color: B,
    pub hovered_foreground_color: C,
    pub background_color: D,
    pub gaps: E,
    pub border: F,
    pub corner_radius: G,
    pub title_height: H,
    pub font_size: I,
    pub text_alignment: J,
    pub initially_expanded: K,
    pub children: Children,
}

impl<Text, A, B, C, D, E, F, G, H, I, J, K, Children> Persistent for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, Children> {
    type Data = CollapsableData;
}

pub struct MyLayouted<C> {
    area: Area,
    children: Option<C>,
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, Children> Element<App> for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, Children>
where
    App: Appli,
    Text: AsRef<str>,
    A: Selector<App, Text>,
    B: Selector<App, App::Color>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, f32>,
    F: Selector<App, f32>,
    G: Selector<App, App::CornerRadius>,
    H: Selector<App, f32>,
    I: Selector<App, App::FontSize>,
    J: Selector<App, HorizontalAlignment>,
    K: Selector<App, bool>,
    Children: ElementSet<App>,
{
    type Layouted = MyLayouted<Children::Layouted>;

    fn make_layout(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::Layouted {
        let persistent = self.get_persistent_data(store, *state.get(&self.initially_expanded));
        let expanded = *persistent.expanded.borrow();

        let title_height = *state.get(&self.title_height);

        let (area, children) = match expanded {
            true => resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
                resolver.push_top(title_height);
                Some(self.children.make_layout(state, store, generator, resolver))
            }),
            false => (resolver.with_height(title_height), None),
        };

        MyLayouted { area, children }
    }

    fn create_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layouted: &'a Self::Layouted,
        layout: &mut Layout<'a, App>,
    ) {
        // TODO: Very much temp
        layout.push_layer();

        if let Some(layouted) = &layouted.children {
            self.children.create_layout(state, store, layouted, layout);
        }

        // TODO: Very much temp
        layout.pop_layer();

        let title_height = *state.get(&self.title_height);

        let title_area = Area {
            x: layouted.area.x,
            y: layouted.area.y,
            width: layouted.area.width,
            height: title_height,
        };

        layout.add_rectangle(
            layouted.area,
            *state.get(&self.corner_radius),
            *state.get(&self.background_color),
        );

        let is_title_hovered = layout.is_area_hovered_and_active(title_area);

        if is_title_hovered {
            let persistent = self.get_persistent_data(store, *state.get(&self.initially_expanded));
            layout.add_toggle(title_area, &persistent.expanded);
            layout.mark_hovered();
        }

        let foreground_color = match is_title_hovered {
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        layout.add_text(
            title_area,
            state.get(&self.text).as_ref(),
            *state.get(&self.font_size),
            foreground_color,
            *state.get(&self.text_alignment),
            *state.get(&theme().collapsable().vertical_alignment()),
        );
    }
}
