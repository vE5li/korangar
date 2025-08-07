use std::cell::RefCell;
use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::Application;
use crate::element::store::{ElementStore, ElementStoreMut, Persistent, PersistentData, PersistentExt};
use crate::element::{Element, ElementSet};
use crate::layout::alignment::{HorizontalAlignment, OverflowBehavior, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Icon, Layout, Resolver};
use crate::theme::{ThemePathGetter, theme};

const CHILDREN_STORE_ID: u64 = 0;
const EXTRA_STORE_ID: u64 = 1;

#[derive(RustState)]
pub struct CollapsableTheme<App>
where
    App: Application,
{
    pub foreground_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub background_color: App::Color,
    pub secondary_background_color: App::Color,
    pub icon_color: App::Color,
    pub icon_size: f32,
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

pub struct Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, Children> {
    pub text_marker: PhantomData<Text>,
    pub text: A,
    pub foreground_color: B,
    pub hovered_foreground_color: C,
    pub background_color: D,
    pub secondary_background_color: E,
    pub icon_color: F,
    pub icon_size: G,
    pub gaps: H,
    pub border: I,
    pub corner_radius: J,
    pub title_height: K,
    pub font_size: L,
    pub text_alignment: M,
    pub initially_expanded: N,
    pub extra_elements: O,
    pub children: Children,
}

impl<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, Children> Persistent
    for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, Children>
{
    type Data = CollapsableData;
}

pub struct MyLayoutInfo<C, E> {
    area: Area,
    children: Option<C>,
    extra_elements: E,
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, Children> Element<App>
    for Collapsable<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, Children>
where
    App: Application,
    Text: AsRef<str>,
    A: Selector<App, Text>,
    B: Selector<App, App::Color>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, f32>,
    H: Selector<App, f32>,
    I: Selector<App, f32>,
    J: Selector<App, App::CornerRadius>,
    K: Selector<App, f32>,
    L: Selector<App, App::FontSize>,
    M: Selector<App, HorizontalAlignment>,
    N: Selector<App, bool>,
    O: ElementSet<App>,
    Children: ElementSet<App>,
{
    type LayoutInfo = MyLayoutInfo<Children::LayoutInfo, O::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        mut store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo {
        let persistent = self.get_persistent_data(&store, *state.get(&self.initially_expanded));
        let expanded = *persistent.expanded.borrow();

        let title_height = *state.get(&self.title_height);
        let fallback_resolver = resolver.clone();

        let (mut area, children) = match expanded {
            true => resolver.with_derived_borderless(*state.get(&self.gaps), *state.get(&self.border), 0.0, |resolver| {
                resolver.push_top(title_height);

                // We need to create a separate store so that the children and the extra
                // elements don't interfere. We need to make sure they both have
                // different ids.
                let children_store = store.child_store(CHILDREN_STORE_ID);

                Some(self.children.create_layout_info(state, children_store, resolver))
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

        // TODO: Figure out a better way to space the elements from the right.
        let extra_space = 40.0;
        let extra_area = Area {
            left: area.left + area.width - extra_space,
            top: area.top,
            width: extra_space,
            height: title_height,
        };
        let mut extra_resolver = Resolver::new(extra_area, 0.0, resolver.get_text_layouter());

        // We need to create a separate store so that the children and the extra
        // elements don't interfere. We need to make sure they both have
        // different ids.
        let extra_store = store.child_store(EXTRA_STORE_ID);
        let extra_elements = self.extra_elements.create_layout_info(state, extra_store, &mut extra_resolver);

        MyLayoutInfo {
            area,
            children,
            extra_elements,
        }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        let use_secondary_color = layout.with_secondary_background(|layout| {
            if let Some(layout_info) = &layout_info.children {
                layout.with_layer(|layout| {
                    let children_store = store.child_store(CHILDREN_STORE_ID);
                    self.children.lay_out(state, children_store, layout_info, layout);
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

        let icon_size = *state.get(&self.icon_size);
        let icon_spacing = (title_area.height - icon_size) / 2.0;

        let icon_area = Area {
            left: title_area.left + icon_spacing * 2.0,
            top: title_area.top + icon_spacing,
            width: icon_size,
            height: icon_size,
        };

        layout.add_icon(
            icon_area,
            Icon::ExpandArrow {
                expanded: layout_info.children.is_some(),
            },
            *state.get(&self.icon_color),
        );

        layout.with_layer(|layout| {
            let extra_store = store.child_store(EXTRA_STORE_ID);
            self.extra_elements.lay_out(state, extra_store, &layout_info.extra_elements, layout);
        });

        let is_title_hovered = layout.is_area_hovered_and_active(title_area);

        if is_title_hovered {
            let persistent = self.get_persistent_data(&store, *state.get(&self.initially_expanded));
            layout.add_toggle(title_area, &persistent.expanded);
            layout.mark_hovered();
        }

        let icon_offset = icon_size + icon_spacing * 4.0;
        let text_area = Area {
            left: title_area.left + icon_offset,
            top: title_area.top,
            height: title_area.height,
            width: title_area.width - icon_offset,
        };

        let foreground_color = match is_title_hovered {
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        layout.add_text(
            text_area,
            state.get(&self.text).as_ref(),
            *state.get(&self.font_size),
            foreground_color,
            *state.get(&self.text_alignment),
            *state.get(&theme().collapsable().vertical_alignment()),
            OverflowBehavior::Shrink,
        );
    }
}
