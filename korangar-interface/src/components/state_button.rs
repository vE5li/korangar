use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, Size};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::event::ClickHandler;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::tooltip::TooltipExt;
use crate::layout::{Icon, MouseButton, Resolvers, WindowLayout, with_single_resolver};

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct StateButtonTheme<App>
where
    App: Application,
{
    pub foreground_color: App::Color,
    pub background_color: App::Color,
    pub highlight_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub hovered_background_color: App::Color,
    pub disabled_foreground_color: App::Color,
    pub disabled_background_color: App::Color,
    pub checkbox_color: App::Color,
    pub hovered_checkbox_color: App::Color,
    pub disabled_checkbox_color: App::Color,
    pub shadow_color: App::Color,
    pub shadow_padding: App::ShadowPadding,
    pub height: f32,
    pub corner_diameter: App::CornerDiameter,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct StateButton<Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X> {
    text_marker: PhantomData<(Text, Tooltip, DisabledTooltip)>,
    text: A,
    tooltip: B,
    state: C,
    event: D,
    disabled: E,
    disabled_tooltip: F,
    foreground_color: G,
    background_color: H,
    highlight_color: I,
    hovered_foreground_color: J,
    hovered_background_color: K,
    disabled_foreground_color: L,
    disabled_background_color: M,
    checkbox_color: N,
    hovered_checkbox_color: O,
    disabled_checkbox_color: P,
    shadow_color: Q,
    shadow_padding: R,
    height: S,
    corner_diameter: T,
    font_size: U,
    horizontal_alignment: V,
    vertical_alignment: W,
    overflow_behavior: X,
}

impl<Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X>
    StateButton<Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X>
{
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        text: A,
        tooltip: B,
        state: C,
        event: D,
        disabled: E,
        disabled_tooltip: F,
        foreground_color: G,
        background_color: H,
        highlight_color: I,
        hovered_foreground_color: J,
        hovered_background_color: K,
        disabled_foreground_color: L,
        disabled_background_color: M,
        checkbox_color: N,
        hovered_checkbox_color: O,
        disabled_checkbox_color: P,
        shadow_color: Q,
        shadow_padding: R,
        height: S,
        corner_diameter: T,
        font_size: U,
        horizontal_alignment: V,
        vertical_alignment: W,
        overflow_behavior: X,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            text,
            tooltip,
            state,
            event,
            disabled,
            disabled_tooltip,
            foreground_color,
            background_color,
            highlight_color,
            hovered_foreground_color,
            hovered_background_color,
            disabled_foreground_color,
            disabled_background_color,
            checkbox_color,
            hovered_checkbox_color,
            disabled_checkbox_color,
            shadow_color,
            shadow_padding,
            height,
            corner_diameter,
            font_size,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
        }
    }
}

impl<App, Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X> Element<App>
    for StateButton<Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Tooltip: AsRef<str> + 'static,
    DisabledTooltip: AsRef<str> + 'static,
    A: Selector<App, Text>,
    B: Selector<App, Tooltip>,
    C: Selector<App, bool>,
    D: ClickHandler<App> + 'static,
    E: Selector<App, bool>,
    F: Selector<App, DisabledTooltip>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, App::Color>,
    J: Selector<App, App::Color>,
    K: Selector<App, App::Color>,
    L: Selector<App, App::Color>,
    M: Selector<App, App::Color>,
    N: Selector<App, App::Color>,
    O: Selector<App, App::Color>,
    P: Selector<App, App::Color>,
    Q: Selector<App, App::Color>,
    R: Selector<App, App::ShadowPadding>,
    S: Selector<App, f32>,
    T: Selector<App, App::CornerDiameter>,
    U: Selector<App, App::FontSize>,
    V: Selector<App, HorizontalAlignment>,
    W: Selector<App, VerticalAlignment>,
    X: Selector<App, App::OverflowBehavior>,
{
    fn create_layout_info(&mut self, state: &Context<App>, _: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let height = *state.get(&self.height);

            let text = state.get(&self.text).as_ref();
            let font_color = *state.get(&self.foreground_color);
            let highlight_color = *state.get(&self.highlight_color);
            let font_size = *state.get(&self.font_size);
            let horizontal_alignment = *state.get(&self.horizontal_alignment);
            let overflow_behavior = *state.get(&self.overflow_behavior);

            let (size, font_size) = resolver.get_text_dimensions(
                text,
                font_color,
                highlight_color,
                font_size,
                horizontal_alignment,
                overflow_behavior,
            );

            let area = resolver.with_height(height.max(size.height()));

            Self::LayoutInfo { area, font_size }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let is_hoverered = layout_info.area.check().run(layout);
        let is_disabled = *state.get(&self.disabled);

        if is_hoverered {
            struct StateButtonTooltip;

            let tooltip = state.get(&self.tooltip).as_ref();
            if !tooltip.is_empty() {
                layout.add_tooltip(tooltip, StateButtonTooltip.tooltip_id());
            }

            if is_disabled {
                let disabled_tooltip = state.get(&self.disabled_tooltip).as_ref();
                if !disabled_tooltip.is_empty() {
                    layout.add_tooltip(disabled_tooltip, StateButtonTooltip.tooltip_id());
                }
            } else {
                layout.register_click_handler(MouseButton::Left, &self.event);
            }
        }

        let background_color = match is_hoverered {
            _ if is_disabled => *state.get(&self.disabled_background_color),
            true => *state.get(&self.hovered_background_color),
            false => *state.get(&self.background_color),
        };

        layout.add_rectangle(
            layout_info.area,
            *state.get(&self.corner_diameter),
            background_color,
            *state.get(&self.shadow_color),
            *state.get(&self.shadow_padding),
        );

        let foreground_color = match is_hoverered {
            _ if is_disabled => *state.get(&self.disabled_foreground_color),
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        layout.add_text(
            layout_info.area,
            state.get(&self.text).as_ref(),
            layout_info.font_size,
            foreground_color,
            *state.get(&self.highlight_color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );

        let checkbox_size = layout_info.area.height - 6.0;
        let checkbox_color = match is_hoverered {
            _ if is_disabled => *state.get(&self.disabled_checkbox_color),
            true => *state.get(&self.hovered_checkbox_color),
            false => *state.get(&self.checkbox_color),
        };

        layout.add_icon(
            Area {
                left: layout_info.area.left + 8.0,
                top: layout_info.area.top + 3.0,
                width: checkbox_size,
                height: checkbox_size,
            },
            Icon::Checkbox {
                checked: *state.get(&self.state),
            },
            checkbox_color,
        );
    }
}
