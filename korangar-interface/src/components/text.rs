use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, Size};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::{Resolver, WindowLayout};

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextTheme<App>
where
    App: Application + 'static,
{
    pub color: App::Color,
    pub highlight_color: App::Color,
    pub height: f32,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct Text<T, A, B, C, D, E, F, G, H> {
    text_marker: PhantomData<T>,
    text: A,
    color: B,
    highlight_color: C,
    height: D,
    font_size: E,
    horizontal_alignment: F,
    vertical_alignment: G,
    overflow_behavior: H,
}

impl<T, A, B, C, D, E, F, G, H> Text<T, A, B, C, D, E, F, G, H> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        text: A,
        color: B,
        highlight_color: C,
        height: D,
        font_size: E,
        horizontal_alignment: F,
        vertical_alignment: G,
        overflow_behavior: H,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            text,
            color,
            highlight_color,
            height,
            font_size,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
        }
    }
}

impl<App, T, A, B, C, D, E, F, G, H> Element<App> for Text<T, A, B, C, D, E, F, G, H>
where
    App: Application,
    T: AsRef<str> + 'static,
    A: Selector<App, T>,
    B: Selector<App, App::Color>,
    C: Selector<App, App::Color>,
    D: Selector<App, f32>,
    E: Selector<App, App::FontSize>,
    F: Selector<App, HorizontalAlignment>,
    G: Selector<App, VerticalAlignment>,
    H: Selector<App, App::OverflowBehavior>,
{
    fn create_layout_info(&mut self, state: &Context<App>, _: ElementStoreMut<'_>, resolver: &mut Resolver<'_, App>) -> Self::LayoutInfo {
        let height = *state.get(&self.height);

        let (size, font_size) = resolver.get_text_dimensions(
            state.get(&self.text).as_ref(),
            *state.get(&self.color),
            *state.get(&self.highlight_color),
            *state.get(&self.font_size),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.overflow_behavior),
        );
        let area = resolver.with_height(height.max(size.height()));

        Self::LayoutInfo { area, font_size }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        layout.add_text(
            layout_info.area,
            state.get(&self.text).as_ref(),
            layout_info.font_size,
            *state.get(&self.color),
            *state.get(&self.highlight_color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}
