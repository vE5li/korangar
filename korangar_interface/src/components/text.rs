use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::Appli;
use crate::element::Element;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::{Layout, Resolver};

#[derive(RustState)]
pub struct TextTheme<App>
where
    App: Appli + 'static,
{
    pub color: App::Color,
    pub height: f32,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
}

pub struct Text<T, A, B, C, D, E, F> {
    pub text_marker: PhantomData<T>,
    pub text: A,
    pub color: B,
    pub height: C,
    pub font_size: D,
    pub horizontal_alignment: E,
    pub vertical_alignment: F,
}

impl<App, T, A, B, C, D, E, F> Element<App> for Text<T, A, B, C, D, E, F>
where
    App: Appli,
    T: AsRef<str> + 'static,
    A: Selector<App, T>,
    B: Selector<App, App::Color>,
    C: Selector<App, f32>,
    D: Selector<App, App::FontSize>,
    E: Selector<App, HorizontalAlignment>,
    F: Selector<App, VerticalAlignment>,
{
    fn make_layout(
        &mut self,
        state: &Context<App>,
        _: &mut ElementStore,
        _: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::Layouted {
        let height = state.get(&self.height);
        let area = resolver.with_height(*height);
        Self::Layouted { area }
    }

    fn create_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        _: &'a ElementStore,
        layouted: &'a Self::Layouted,
        layout: &mut Layout<'a, App>,
    ) {
        layout.add_text(
            layouted.area,
            state.get(&self.text).as_ref(),
            *state.get(&self.font_size),
            *state.get(&self.color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
        );
    }
}
