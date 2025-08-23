use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::{Application, Size};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::{Resolver, WindowLayout};

#[derive(RustState)]
pub struct TextTheme<App>
where
    App: Application + 'static,
{
    pub color: App::Color,
    pub height: f32,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct Text<T, A, B, C, D, E, F, G> {
    text_marker: PhantomData<T>,
    text: A,
    color: B,
    height: C,
    font_size: D,
    horizontal_alignment: E,
    vertical_alignment: F,
    overflow_behavior: G,
}

impl<T, A, B, C, D, E, F, G> Text<T, A, B, C, D, E, F, G> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    pub fn component_new(
        text: A,
        color: B,
        height: C,
        font_size: D,
        horizontal_alignment: E,
        vertical_alignment: F,
        overflow_behavior: G,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            text,
            color,
            height,
            font_size,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
        }
    }
}

impl<App, T, A, B, C, D, E, F, G> Element<App> for Text<T, A, B, C, D, E, F, G>
where
    App: Application,
    T: AsRef<str> + 'static,
    A: Selector<App, T>,
    B: Selector<App, App::Color>,
    C: Selector<App, f32>,
    D: Selector<App, App::FontSize>,
    E: Selector<App, HorizontalAlignment>,
    F: Selector<App, VerticalAlignment>,
    G: Selector<App, App::OverflowBehavior>,
{
    fn create_layout_info(&mut self, state: &Context<App>, _: ElementStoreMut<'_>, resolver: &mut Resolver<'_, App>) -> Self::LayoutInfo {
        let height = *state.get(&self.height);

        let (size, font_size) = resolver.get_text_dimensions(
            state.get(&self.text).as_ref(),
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
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}
