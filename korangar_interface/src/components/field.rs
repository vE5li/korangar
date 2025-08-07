use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, SizeTrait};
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::element::{DefaultLayoutInfoWithText, Element};
use crate::layout::alignment::{HorizontalAlignment, OverflowBehavior, VerticalAlignment};
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
}

pub struct Field<Text, Tooltip, A, B, C, D, E, F, G, H, I> {
    pub text_marker: PhantomData<(Text, Tooltip)>,
    pub text: A,
    pub tooltip: B,
    pub foreground_color: C,
    pub background_color: D,
    pub height: E,
    pub corner_radius: F,
    pub font_size: G,
    pub horizontal_alignment: H,
    pub vertical_alignment: I,
}

impl<App, Text, Tooltip, A, B, C, D, E, F, G, H, I> Element<App> for Field<Text, Tooltip, A, B, C, D, E, F, G, H, I>
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
{
    type LayoutInfo = DefaultLayoutInfoWithText<App>;

    fn create_layout_info(&mut self, state: &Context<App>, _: ElementStoreMut<'_>, resolver: &mut Resolver<'_, App>) -> Self::LayoutInfo {
        let height = *state.get(&self.height);

        let text = state.get(&self.text).as_ref();
        let font_size = *state.get(&self.font_size);
        let horizontal_alignment = *state.get(&self.horizontal_alignment);
        let overflow_behavior = OverflowBehavior::LineBreak;

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
            OverflowBehavior::LineBreak,
        );
    }
}
