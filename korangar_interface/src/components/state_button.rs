use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, SizeTrait};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::event::ClickAction;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Icon, Layout, MouseButton, Resolver};

#[derive(RustState)]
pub struct StateButtonTheme<App>
where
    App: Application,
{
    pub foreground_color: App::Color,
    pub background_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub hovered_background_color: App::Color,
    pub checkbox_color: App::Color,
    pub height: f32,
    pub corner_radius: App::CornerRadius,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct StateButton<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> {
    text_marker: PhantomData<Text>,
    text: A,
    state: B,
    event: C,
    disabled: D,
    foreground_color: E,
    background_color: F,
    hovered_foreground_color: G,
    hovered_background_color: H,
    checkbox_color: I,
    height: J,
    corner_radius: K,
    font_size: L,
    horizontal_alignment: M,
    vertical_alignment: N,
    overflow_behavior: O,
}

impl<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> StateButton<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        text: A,
        state: B,
        event: C,
        disabled: D,
        foreground_color: E,
        background_color: F,
        hovered_foreground_color: G,
        hovered_background_color: H,
        checkbox_color: I,
        height: J,
        corner_radius: K,
        font_size: L,
        horizontal_alignment: M,
        vertical_alignment: N,
        overflow_behavior: O,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            text,
            state,
            event,
            disabled,
            foreground_color,
            background_color,
            hovered_foreground_color,
            hovered_background_color,
            checkbox_color,
            height,
            corner_radius,
            font_size,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
        }
    }
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> Element<App> for StateButton<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O>
where
    App: Application,
    Text: AsRef<str> + 'static,
    A: Selector<App, Text>,
    B: Selector<App, bool>,
    C: ClickAction<App> + 'static,
    D: Selector<App, bool>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, App::Color>,
    J: Selector<App, f32>,
    K: Selector<App, App::CornerRadius>,
    L: Selector<App, App::FontSize>,
    M: Selector<App, HorizontalAlignment>,
    N: Selector<App, VerticalAlignment>,
    O: Selector<App, App::OverflowBehavior>,
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

        let checkbox_size = layout_info.area.height - 6.0;

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
            *state.get(&self.checkbox_color),
        );
    }
}
