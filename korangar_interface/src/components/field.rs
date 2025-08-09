use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, SizeTrait};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::tooltip::TooltipExt;
use crate::layout::{Layout, Resolver};

#[derive(RustState)]
pub struct FieldTheme<App>
where
    App: Application + 'static,
{
    pub foreground_color: App::Color,
    pub background_color: App::Color,
    pub height: f32,
    pub corner_radius: App::CornerRadius,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct Field<Text, Tooltip, A, B, C, D, E, F, G, H, I, J> {
    text_marker: PhantomData<(Text, Tooltip)>,
    text: A,
    tooltip: B,
    foreground_color: C,
    background_color: D,
    height: E,
    corner_radius: F,
    font_size: G,
    horizontal_alignment: H,
    vertical_alignment: I,
    overflow_behavior: J,
}

impl<Text, Tooltip, A, B, C, D, E, F, G, H, I, J> Field<Text, Tooltip, A, B, C, D, E, F, G, H, I, J> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        text: A,
        tooltip: B,
        foreground_color: C,
        background_color: D,
        height: E,
        corner_radius: F,
        font_size: G,
        horizontal_alignment: H,
        vertical_alignment: I,
        overflow_behavior: J,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            text,
            tooltip,
            foreground_color,
            background_color,
            height,
            corner_radius,
            font_size,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
        }
    }
}

impl<App, Text, Tooltip, A, B, C, D, E, F, G, H, I, J> Element<App> for Field<Text, Tooltip, A, B, C, D, E, F, G, H, I, J>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Tooltip: AsRef<str> + 'static,
    A: Selector<App, Text>,
    B: Selector<App, Tooltip>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, f32>,
    F: Selector<App, App::CornerRadius>,
    G: Selector<App, App::FontSize>,
    H: Selector<App, HorizontalAlignment>,
    I: Selector<App, VerticalAlignment>,
    J: Selector<App, App::OverflowBehavior>,
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
            layout.mark_hovered();

            let tooltip = state.get(&self.tooltip).as_ref();

            if !tooltip.is_empty() {
                struct FieldTooltip;
                layout.add_tooltip(tooltip, FieldTooltip.tooltip_id());
            }
        }

        layout.add_rectangle(
            layout_info.area,
            *state.get(&self.corner_radius),
            *state.get(&self.background_color),
        );
        layout.add_text(
            layout_info.area,
            state.get(&self.text).as_ref(),
            layout_info.font_size,
            *state.get(&self.foreground_color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}
