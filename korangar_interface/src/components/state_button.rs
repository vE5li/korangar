use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::Application;
use crate::element::Element;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::event::ClickAction;
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Layout, MouseButton, Resolver};
use crate::theme::{ThemePathGetter, theme};

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
    pub text_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
}

pub struct StateButton<Text, A, B, C, D, E, F, G, H, I, J, K, L, M> {
    pub text_marker: PhantomData<Text>,
    pub text: A,
    pub state: B,
    pub event: C,
    pub disabled: D,
    pub foreground_color: E,
    pub background_color: F,
    pub hovered_foreground_color: G,
    pub hovered_background_color: H,
    pub checkbox_color: I,
    pub height: J,
    pub corner_radius: K,
    pub font_size: L,
    pub text_alignment: M,
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L, M> Element<App> for StateButton<Text, A, B, C, D, E, F, G, H, I, J, K, L, M>
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
            *state.get(&theme().state_button().vertical_alignment()),
        );

        let checkbox_size = layout_info.area.height - 6.0;
        layout.add_checkbox(
            Area {
                x: layout_info.area.x + 8.0,
                y: layout_info.area.y + 3.0,
                width: checkbox_size,
                height: checkbox_size,
            },
            *state.get(&self.checkbox_color),
            *state.get(&self.state),
        );
    }
}
