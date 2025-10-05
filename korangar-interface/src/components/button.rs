use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, Size};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::event::ClickHandler;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::tooltip::TooltipExt;
use crate::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ButtonTheme<App>
where
    App: Application + 'static,
{
    pub foreground_color: App::Color,
    pub background_color: App::Color,
    pub highlight_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub hovered_background_color: App::Color,
    pub disabled_foreground_color: App::Color,
    pub disabled_background_color: App::Color,
    pub shadow_color: App::Color,
    pub shadow_padding: App::ShadowPadding,
    pub height: f32,
    pub corner_diameter: App::CornerDiameter,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct Button<Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T> {
    text_marker: PhantomData<(Text, Tooltip, DisabledTooltip)>,
    text: A,
    tooltip: B,
    event: C,
    disabled: D,
    disabled_tooltip: E,
    foreground_color: F,
    background_color: G,
    highlight_color: H,
    hovered_foreground_color: I,
    hovered_background_color: J,
    disabled_foreground_color: K,
    disabled_background_color: L,
    shadow_color: M,
    shadow_padding: N,
    height: O,
    corner_diameter: P,
    font_size: Q,
    horizontal_alignment: R,
    vertical_alignment: S,
    overflow_behavior: T,
}

impl<Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T>
    Button<Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T>
{
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        text: A,
        tooltip: B,
        event: C,
        disabled: D,
        disabled_tooltip: E,
        foreground_color: F,
        background_color: G,
        highlight_color: H,
        hovered_foreground_color: I,
        hovered_background_color: J,
        disabled_foreground_color: K,
        disabled_background_color: L,
        shadow_color: M,
        shadow_padding: N,
        height: O,
        corner_diameter: P,
        font_size: Q,
        horizontal_alignment: R,
        vertical_alignment: S,
        overflow_behavior: T,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            text,
            tooltip,
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

impl<App, Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T> Element<App>
    for Button<Text, Tooltip, DisabledTooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Tooltip: AsRef<str> + 'static,
    DisabledTooltip: AsRef<str> + 'static,
    A: Selector<App, Text>,
    B: Selector<App, Tooltip>,
    C: ClickHandler<App> + 'static,
    D: Selector<App, bool>,
    E: Selector<App, DisabledTooltip>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, App::Color>,
    J: Selector<App, App::Color>,
    K: Selector<App, App::Color>,
    L: Selector<App, App::Color>,
    M: Selector<App, App::Color>,
    N: Selector<App, App::ShadowPadding>,
    O: Selector<App, f32>,
    P: Selector<App, App::CornerDiameter>,
    Q: Selector<App, App::FontSize>,
    R: Selector<App, HorizontalAlignment>,
    S: Selector<App, VerticalAlignment>,
    T: Selector<App, App::OverflowBehavior>,
{
    fn create_layout_info(&mut self, state: &Context<App>, _: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let height = *state.get(&self.height);

            let text = state.get(&self.text).as_ref();
            let font_size = *state.get(&self.font_size);
            let horizontal_alignment = *state.get(&self.horizontal_alignment);
            let overflow_behavior = *state.get(&self.overflow_behavior);
            let foreground_color = *state.get(&self.foreground_color);
            let highlight_color = *state.get(&self.highlight_color);

            let (size, font_size) = resolver.get_text_dimensions(
                text,
                foreground_color,
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
            struct ButtonTooltip;

            let tooltip = state.get(&self.tooltip).as_ref();
            if !tooltip.is_empty() {
                layout.add_tooltip(tooltip, ButtonTooltip.tooltip_id());
            }

            if is_disabled {
                let disabled_tooltip = state.get(&self.disabled_tooltip).as_ref();
                if !disabled_tooltip.is_empty() {
                    layout.add_tooltip(disabled_tooltip, ButtonTooltip.tooltip_id());
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
    }
}
