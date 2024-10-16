use std::cell::RefCell;
use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, Size};
use crate::element::store::{ElementStore, ElementStoreMut, Persistent, PersistentData, PersistentExt};
use crate::element::{Element, ElementSet};
use crate::event::ClickHandler;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::tooltip::TooltipExt;
use crate::layout::{Icon, MouseButton, Resolver, WindowLayout};
use crate::prelude::EventQueue;

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
    pub corner_diameter: App::CornerDiameter,
    pub title_height: f32,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct CollapsableData {
    expanded: RefCell<bool>,
}

impl PersistentData for CollapsableData {
    type Inputs = bool;

    fn from_inputs(inputs: Self::Inputs) -> Self {
        Self {
            expanded: RefCell::new(inputs),
        }
    }
}

impl<App> ClickHandler<App> for CollapsableData
where
    App: Application,
{
    fn execute(&self, _: &Context<App>, _: &mut EventQueue<App>) {
        let mut expanded = self.expanded.borrow_mut();
        *expanded = !*expanded;
    }
}

impl<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Children> Persistent
    for Collapsable<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Children>
{
    type Data = CollapsableData;
}

pub struct CollapseableLayoutInfo<App, C, E>
where
    App: Application,
{
    area: Area,
    title_height: f32,
    expanded: bool,
    font_size: App::FontSize,
    children: Option<C>,
    extra_elements: E,
}

pub struct Collapsable<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Children> {
    text_marker: PhantomData<(Text, Tooltip)>,
    text: A,
    tooltip: B,
    foreground_color: C,
    hovered_foreground_color: D,
    background_color: E,
    secondary_background_color: F,
    icon_color: G,
    icon_size: H,
    gaps: I,
    border: J,
    corner_diameter: K,
    title_height: L,
    font_size: M,
    horizontal_alignment: N,
    vertical_alignment: O,
    overflow_behavior: P,
    initially_expanded: Q,
    extra_elements: R,
    children: Children,
}

impl<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Children>
    Collapsable<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Children>
{
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        text: A,
        tooltip: B,
        foreground_color: C,
        hovered_foreground_color: D,
        background_color: E,
        secondary_background_color: F,
        icon_color: G,
        icon_size: H,
        gaps: I,
        border: J,
        corner_diameter: K,
        title_height: L,
        font_size: M,
        horizontal_alignment: N,
        vertical_alignment: O,
        overflow_behavior: P,
        initially_expanded: Q,
        extra_elements: R,
        children: Children,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            text,
            tooltip,
            foreground_color,
            hovered_foreground_color,
            background_color,
            secondary_background_color,
            icon_color,
            icon_size,
            gaps,
            border,
            corner_diameter,
            title_height,
            font_size,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
            initially_expanded,
            extra_elements,
            children,
        }
    }
}

impl<App, Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Children> Element<App>
    for Collapsable<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, Children>
where
    App: Application,
    Text: AsRef<str>,
    Tooltip: AsRef<str>,
    A: Selector<App, Text>,
    B: Selector<App, Tooltip>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, f32>,
    I: Selector<App, f32>,
    J: Selector<App, f32>,
    K: Selector<App, App::CornerDiameter>,
    L: Selector<App, f32>,
    M: Selector<App, App::FontSize>,
    N: Selector<App, HorizontalAlignment>,
    O: Selector<App, VerticalAlignment>,
    P: Selector<App, App::OverflowBehavior>,
    Q: Selector<App, bool>,
    R: ElementSet<App>,
    Children: ElementSet<App>,
{
    type LayoutInfo = CollapseableLayoutInfo<App, Children::LayoutInfo, R::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        mut store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo {
        let persistent = self.get_persistent_data(&store, *state.get(&self.initially_expanded));
        let expanded = *persistent.expanded.borrow();

        let text = state.get(&self.text).as_ref();
        let font_size = *state.get(&self.font_size);
        let horizontal_alignment = *state.get(&self.horizontal_alignment);
        let overflow_behavior = *state.get(&self.overflow_behavior);

        let (size, font_size) = resolver.get_text_dimensions(text, font_size, horizontal_alignment, overflow_behavior);

        let title_height = state.get(&self.title_height).max(size.height());

        let (area, children) = match expanded && self.children.get_element_count(state) > 0 {
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

        Self::LayoutInfo {
            area,
            title_height,
            expanded,
            font_size,
            children,
            extra_elements,
        }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let use_secondary_color = layout.with_secondary_background(|layout| {
            if let Some(layout_info) = &layout_info.children {
                layout.with_layer(|layout| {
                    let children_store = store.child_store(CHILDREN_STORE_ID);
                    self.children.lay_out(state, children_store, layout_info, layout);
                });
            }
        });

        let title_area = Area {
            left: layout_info.area.left,
            top: layout_info.area.top,
            width: layout_info.area.width,
            height: layout_info.title_height,
        };

        let background_color = match use_secondary_color {
            true => *state.get(&self.secondary_background_color),
            false => *state.get(&self.background_color),
        };

        layout.add_rectangle(layout_info.area, *state.get(&self.corner_diameter), background_color);

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
                expanded: layout_info.expanded,
            },
            *state.get(&self.icon_color),
        );

        layout.with_layer(|layout| {
            let extra_store = store.child_store(EXTRA_STORE_ID);
            self.extra_elements.lay_out(state, extra_store, &layout_info.extra_elements, layout);
        });

        let is_title_hovered = title_area.check().run(layout);

        if is_title_hovered {
            let tooltip = state.get(&self.tooltip).as_ref();
            if !tooltip.is_empty() {
                struct CollapsableTooltip;

                layout.add_tooltip(tooltip, CollapsableTooltip.tooltip_id());
            }

            let persistent = self.get_persistent_data(&store, *state.get(&self.initially_expanded));
            layout.add_click_area(title_area, MouseButton::Left, persistent);
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
            layout_info.font_size,
            foreground_color,
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}
