use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::Application;
use crate::element::Element;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::event::ClickAction;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::{Layout, MouseButton, Resolver};
use crate::theme::{ThemePathGetter, theme};

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
    pub text_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
}

pub struct Button<Text, A, B, C, D, E, F, G, H, I, J, K> {
    pub text_marker: PhantomData<Text>,
    pub text: A,
    pub event: B,
    pub disabled: C,
    pub foreground_color: D,
    pub background_color: E,
    pub hovered_foreground_color: F,
    pub hovered_background_color: G,
    pub height: H,
    pub corner_radius: I,
    pub font_size: J,
    pub text_alignment: K,
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K> Element<App> for Button<Text, A, B, C, D, E, F, G, H, I, J, K>
where
    App: Application,
    Text: AsRef<str> + 'static,
    A: Selector<App, Text>,
    B: ClickAction<App> + 'static,
    C: Selector<App, bool>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, f32>,
    I: Selector<App, App::CornerRadius>,
    J: Selector<App, App::FontSize>,
    K: Selector<App, HorizontalAlignment>,
{
    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
        let height = state.get(&self.height);
        let area = resolver.with_height(*height);
        Self::LayoutInfo { area }
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
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
            *state.get(&self.font_size),
            foreground_color,
            *state.get(&self.text_alignment),
            *state.get(&theme().button().vertical_alignment()),
        );
    }
}
