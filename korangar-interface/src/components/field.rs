use std::marker::PhantomData;

use rust_state::{RustState, Selector, State};

use crate::application::{Application, Size};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::tooltip::TooltipExt;
use crate::layout::{Resolvers, WindowLayout, with_single_resolver};

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FieldTheme<App>
where
    App: Application + 'static,
{
    pub foreground_color: App::Color,
    pub background_color: App::Color,
    pub highlight_color: App::Color,
    pub shadow_color: App::Color,
    pub shadow_padding: App::ShadowPadding,
    pub height: f32,
    pub corner_diameter: App::CornerDiameter,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct Field<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M> {
    text_marker: PhantomData<(Text, Tooltip)>,
    text: A,
    tooltip: B,
    foreground_color: C,
    background_color: D,
    highlight_color: E,
    shadow_color: F,
    shadow_padding: G,
    height: H,
    corner_diameter: I,
    font_size: J,
    horizontal_alignment: K,
    vertical_alignment: L,
    overflow_behavior: M,
}

impl<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M> Field<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        text: A,
        tooltip: B,
        foreground_color: C,
        background_color: D,
        highlight_color: E,
        shadow_color: F,
        shadow_padding: G,
        height: H,
        corner_diameter: I,
        font_size: J,
        horizontal_alignment: K,
        vertical_alignment: L,
        overflow_behavior: M,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            text,
            tooltip,
            foreground_color,
            background_color,
            highlight_color,
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

impl<App, Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M> Element<App> for Field<Text, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M>
where
    App: Application,
    Text: AsRef<str> + 'static,
    Tooltip: AsRef<str> + 'static,
    A: Selector<App, Text>,
    B: Selector<App, Tooltip>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::ShadowPadding>,
    H: Selector<App, f32>,
    I: Selector<App, App::CornerDiameter>,
    J: Selector<App, App::FontSize>,
    K: Selector<App, HorizontalAlignment>,
    L: Selector<App, VerticalAlignment>,
    M: Selector<App, App::OverflowBehavior>,
{
    fn create_layout_info(&mut self, state: &State<App>, _: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let height = *state.get(&self.height);
            let text = state.get(&self.text).as_ref();
            let foreground_color = *state.get(&self.foreground_color);
            let highlight_color = *state.get(&self.highlight_color);
            let font_size = *state.get(&self.font_size);
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

            let area = resolver.with_height(height.max(size.height()));
            Self::LayoutInfo { area, font_size }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        if layout_info.area.check().run(layout) {
            let tooltip = state.get(&self.tooltip).as_ref();

            if !tooltip.is_empty() {
                struct FieldTooltip;
                layout.add_tooltip(tooltip, FieldTooltip.tooltip_id());
            }
        }

        layout.add_rectangle(
            layout_info.area,
            *state.get(&self.corner_diameter),
            *state.get(&self.background_color),
            *state.get(&self.shadow_color),
            *state.get(&self.shadow_padding),
        );
        layout.add_text(
            layout_info.area,
            state.get(&self.text).as_ref(),
            layout_info.font_size,
            *state.get(&self.foreground_color),
            *state.get(&self.highlight_color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}
