use std::cell::RefCell;
use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::Application;
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
    App: Application,
{
    pub foreground_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub background_color: App::Color,
    pub secondary_background_color: App::Color,
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

pub struct Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, Children> {
    pub text_marker: PhantomData<Text>,
    pub text: A,
    pub foreground_color: B,
    pub hovered_foreground_color: C,
    pub background_color: D,
    pub secondary_background_color: E,
    pub gaps: F,
    pub border: G,
    pub corner_radius: H,
    pub title_height: I,
    pub font_size: J,
    pub text_alignment: K,
    pub initially_expanded: L,
    pub extra_elements: M,
    pub children: Children,
}

impl<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, Children> Persistent
    for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, Children>
{
    type Data = CollapsableData;
}

pub struct MyLayoutInfo<C, E> {
    area: Area,
    children: Option<C>,
    extra_elements: E,
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L, M, Children> Element<App>
    for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, Children>
where
    App: Application,
    Text: AsRef<str>,
    A: Selector<App, Text>,
    B: Selector<App, App::Color>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, f32>,
    G: Selector<App, f32>,
    H: Selector<App, App::CornerRadius>,
    I: Selector<App, f32>,
    J: Selector<App, App::FontSize>,
    K: Selector<App, HorizontalAlignment>,
    L: Selector<App, bool>,
    M: ElementSet<App>,
    Children: ElementSet<App>,
{
    type LayoutInfo = MyLayoutInfo<Children::LayoutInfo, M::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
        let persistent = self.get_persistent_data(store, *state.get(&self.initially_expanded));
        let expanded = *persistent.expanded.borrow();

        let title_height = *state.get(&self.title_height);
        let fallback_resolver = resolver.clone();

        let (mut area, children) = match expanded {
            true => resolver.with_derived(*state.get(&self.gaps), *state.get(&self.border), |resolver| {
                resolver.push_top(title_height);
                Some(self.children.create_layout_info(state, store, generator, resolver))
            }),
            false => (resolver.with_height(title_height), None),
        };

        // Special check so that an expanded `Collapsable` without any elements looks
        // the same as a non-expanded one.
        // FIX: Don't do this. Instead just pass get_element_count the same arguments as
        // create_layout_info.
        if expanded && self.children.get_element_count() == 0 {
            *resolver = fallback_resolver;
            area = resolver.with_height(title_height);
        }

        // TODO: Custom resolver set that allocates in the title area
        // FIX: Will use the same store entry as the elements inside. We need to split
        // the store again here.
        let extra_elements = self.extra_elements.create_layout_info(state, store, generator, resolver);

        MyLayoutInfo {
            area,
            children,
            extra_elements,
        }
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        let use_secondary_color = layout.with_secondary_background(|layout| {
            if let Some(layout_info) = &layout_info.children {
                layout.with_layer(|layout| {
                    self.children.layout_element(state, store, layout_info, layout);
                });
            }
        });

        let title_height = *state.get(&self.title_height);

        let title_area = Area {
            left: layout_info.area.left,
            top: layout_info.area.top,
            width: layout_info.area.width,
            height: title_height,
        };

        let background_color = match use_secondary_color {
            true => *state.get(&self.secondary_background_color),
            false => *state.get(&self.background_color),
        };

        layout.add_rectangle(layout_info.area, *state.get(&self.corner_radius), background_color);

        layout.with_layer(|layout| {
            self.extra_elements
                .layout_element(state, store, &layout_info.extra_elements, layout);
        });

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
