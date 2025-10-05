use std::cell::Cell;
use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, Size};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut, Persistent, PersistentData, PersistentExt};
use crate::event::ClickHandler;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::tooltip::TooltipExt;
use crate::layout::{Icon, MouseButton, Resolver, Resolvers, WindowLayout, with_single_resolver};
use crate::prelude::EventQueue;

const CHILDREN_STORE_ID: u64 = 0;
const EXTRA_STORE_ID: u64 = 1;

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CollapsibleTheme<App>
where
    App: Application,
{
    pub foreground_color: App::Color,
    pub highlight_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub background_color: App::Color,
    pub secondary_background_color: App::Color,
    pub icon_color: App::Color,
    pub icon_size: f32,
    pub shadow_color: App::Color,
    pub shadow_padding: App::ShadowPadding,
    pub gaps: f32,
    pub border: f32,
    pub corner_diameter: App::CornerDiameter,
    pub title_height: f32,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct CollapsibleData {
    expanded: Cell<bool>,
}

impl PersistentData for CollapsibleData {
    type Inputs = bool;

    fn from_inputs(inputs: Self::Inputs) -> Self {
        Self {
            expanded: Cell::new(inputs),
        }
    }
}

impl<App> ClickHandler<App> for CollapsibleData
where
    App: Application,
{
    fn handle_click(&self, _: &Context<App>, _: &mut EventQueue<App>) {
        let expanded = self.expanded.get();
        self.expanded.set(!expanded);
    }
}

impl<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Children> Persistent
    for Collapsible<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Children>
{
    type Data = CollapsibleData;
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

pub struct Collapsible<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Children> {
    text_marker: PhantomData<(Text, Tooltip)>,
    text: A,
    tooltip: B,
    foreground_color: C,
    highlight_color: D,
    hovered_foreground_color: E,
    background_color: F,
    secondary_background_color: G,
    icon_color: H,
    icon_size: I,
    shadow_color: J,
    shadow_padding: K,
    gaps: L,
    border: M,
    corner_diameter: N,
    title_height: O,
    font_size: P,
    horizontal_alignment: Q,
    vertical_alignment: R,
    overflow_behavior: S,
    initially_expanded: T,
    extra_elements: U,
    children: Children,
}

impl<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Children>
    Collapsible<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Children>
{
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        text: A,
        tooltip: B,
        foreground_color: C,
        highlight_color: D,
        hovered_foreground_color: E,
        background_color: F,
        secondary_background_color: G,
        icon_color: H,
        icon_size: I,
        shadow_color: J,
        shadow_padding: K,
        gaps: L,
        border: M,
        corner_diameter: N,
        title_height: O,
        font_size: P,
        horizontal_alignment: Q,
        vertical_alignment: R,
        overflow_behavior: S,
        initially_expanded: T,
        extra_elements: U,
        children: Children,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            text,
            tooltip,
            foreground_color,
            highlight_color,
            hovered_foreground_color,
            background_color,
            secondary_background_color,
            icon_color,
            icon_size,
            shadow_color,
            shadow_padding,
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

impl<App, Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Children> Element<App>
    for Collapsible<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Children>
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
    H: Selector<App, App::Color>,
    I: Selector<App, f32>,
    J: Selector<App, App::Color>,
    K: Selector<App, App::ShadowPadding>,
    L: Selector<App, f32>,
    M: Selector<App, f32>,
    N: Selector<App, App::CornerDiameter>,
    O: Selector<App, f32>,
    P: Selector<App, App::FontSize>,
    Q: Selector<App, HorizontalAlignment>,
    R: Selector<App, VerticalAlignment>,
    S: Selector<App, App::OverflowBehavior>,
    T: Selector<App, bool>,
    U: Element<App>,
    Children: Element<App>,
{
    type LayoutInfo = CollapseableLayoutInfo<App, Children::LayoutInfo, U::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<App>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let persistent = self.get_persistent_data(&store, *state.get(&self.initially_expanded));
            let expanded = persistent.expanded.get();

            let text = state.get(&self.text).as_ref();
            let font_size = *state.get(&self.font_size);
            let foreground_color = *state.get(&self.foreground_color);
            let highlight_color = *state.get(&self.highlight_color);

            let horizontal_alignment = *state.get(&self.horizontal_alignment);
            let overflow_behavior = *state.get(&self.overflow_behavior);

            let (size, font_size) = resolver.get_text_dimensions(
                text,
                foreground_color,
                highlight_color,
                font_size,
                horizontal_alignment,
                overflow_behavior,
            );

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
            let extra_elements = self.extra_elements.create_layout_info(state, extra_store, &mut extra_resolver as _);

            Self::LayoutInfo {
                area,
                title_height,
                expanded,
                font_size,
                children,
                extra_elements,
            }
        })
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

        layout.add_rectangle(
            layout_info.area,
            *state.get(&self.corner_diameter),
            background_color,
            *state.get(&self.shadow_color),
            *state.get(&self.shadow_padding),
        );

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
                struct CollapsibleTooltip;

                layout.add_tooltip(tooltip, CollapsibleTooltip.tooltip_id());
            }

            let persistent = self.get_persistent_data(&store, *state.get(&self.initially_expanded));
            layout.register_click_handler(MouseButton::Left, persistent);
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
            *state.get(&self.highlight_color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}
