use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, SizeTrait};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::event::ClickAction;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::tooltip::TooltipExt;
use crate::layout::{Layout, MouseButton, Resolver};

#[derive(RustState)]
pub struct ButtonTheme<App>
where
    App: Application + 'static,
{
    pub foreground_color: App::Color,
    pub background_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub hovered_background_color: App::Color,
    pub height: f32,
    pub corner_radius: App::CornerRadius,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct Button<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N> {
    pub text_marker: PhantomData<(Text, Tooltip)>,
    pub text: A,
    pub tooltip: B,
    pub event: C,
    pub disabled: D,
    pub foreground_color: E,
    pub background_color: F,
    pub hovered_foreground_color: G,
    pub hovered_background_color: H,
    pub height: I,
    pub corner_radius: J,
    pub font_size: K,
    pub horizontal_alignment: L,
    pub vertical_alignment: M,
    pub overflow_behavior: N,
}

impl<App, Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N> Element<App>
    for Button<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M, N>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Tooltip: AsRef<str> + 'static,
    A: Selector<App, Text>,
    B: Selector<App, Tooltip>,
    C: ClickAction<App> + 'static,
    D: Selector<App, bool>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, f32>,
    J: Selector<App, App::CornerRadius>,
    K: Selector<App, App::FontSize>,
    L: Selector<App, HorizontalAlignment>,
    M: Selector<App, VerticalAlignment>,
    N: Selector<App, App::OverflowBehavior>,
{
    fn create_layout_info(&mut self, state: &Context<App>, _: ElementStoreMut<'_>, resolver: &mut Resolver<'_, App>) -> Self::LayoutInfo {
        let height = *state.get(&self.height);

        let text = state.get(&self.text).as_ref();
        let font_size = *state.get(&self.font_size);
        let horizontal_alignment = *state.get(&self.horizontal_alignment);
        let overflow_behavior = *state.get(&self.overflow_behavior);

        let (size, font_size) = resolver.get_text_dimensions(text, font_size, horizontal_alignment, overflow_behavior);

        let area = resolver.with_height(height.max(size.height()));
        Self::LayoutInfo { area, font_size }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        let is_hoverered = layout.is_area_hovered_and_active(layout_info.area);

        if is_hoverered {
            layout.add_click_area(layout_info.area, MouseButton::Left, &self.event);
            layout.mark_hovered();

            let tooltip = state.get(&self.tooltip).as_ref();

            if !tooltip.is_empty() {
                struct ButtonTooltip;
                layout.add_tooltip(tooltip, ButtonTooltip.tooltip_id());
            }
        }

        let background_color = match is_hoverered {
            true => *state.get(&self.hovered_background_color),
            false => *state.get(&self.background_color),
        };

        layout.add_rectangle(layout_info.area, *state.get(&self.corner_radius), background_color);

        let foreground_color = match is_hoverered {
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        layout.add_text(
            layout_info.area,
            state.get(&self.text).as_ref(),
            layout_info.font_size,
            foreground_color,
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}
