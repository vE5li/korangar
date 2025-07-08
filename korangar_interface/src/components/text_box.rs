use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use crate::application::Appli;
use crate::element::id::ElementIdGenerator;
use crate::element::store::{ElementStore, Persistent, PersistentExt};
use crate::element::{Element, ElementSet};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::{HeightBound, InputHandler, Layout, Resolver};
use crate::theme::ThemePathGetter;

#[derive(RustState)]
pub struct TextBoxTheme<App>
where
    App: Appli,
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

pub struct TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L> {
    pub text_marker: PhantomData<Text>,
    pub text: A,
    pub state: B,
    pub input_handler: C,
    pub disabled: D,
    pub foreground_color: E,
    pub background_color: F,
    pub hovered_foreground_color: G,
    pub hovered_background_color: H,
    pub height: I,
    pub corner_radius: J,
    pub font_size: K,
    pub text_alignment: L,
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L> Element<App> for TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L>
where
    App: Appli,
    Text: AsRef<str> + 'static,
    A: Selector<App, Text>,
    B: Selector<App, String>,
    C: InputHandler<App> + 'static,
    D: Selector<App, bool>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, f32>,
    J: Selector<App, App::CornerRadius>,
    K: Selector<App, App::FontSize>,
    L: Selector<App, HorizontalAlignment>,
{
    fn make_layout(
        &mut self,
        state: &Context<App>,
        store: &mut ElementStore,
        generator: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::Layouted {
        let height = state.get(&self.height);

        Self::Layouted {
            area: resolver.with_height(*height),
        }
    }

    fn create_layout<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layouted: &'a Self::Layouted,
        layout: &mut Layout<'a, App>,
    ) {
        let is_hoverered = layout.is_area_hovered_and_active(layouted.area);

        if is_hoverered {
            layout.mark_hovered();
        }

        layout.add_focus_area(layouted.area, store.get_element_id());

        if layout.is_element_focused(store.get_element_id()) {
            layout.add_input_handler(&self.input_handler);
        }

        let background_color = match is_hoverered {
            true => *state.get(&self.hovered_background_color),
            false => *state.get(&self.background_color),
        };

        // TODO: Remove if
        if !layout.is_element_focused(store.get_element_id()) {
            layout.add_rectangle(layouted.area, *state.get(&self.corner_radius), background_color);
        }

        let foreground_color = match is_hoverered {
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        // layout.add_text(
        //     area,
        //     state.get(&self.text).as_ref(),
        //     *state.get(&self.font_size),
        //     foreground_color,
        //     *state.get(&self.text_alignment),
        //     *state.get(&theme().text_box().vertical_alignment()),
        // );

        layout.add_text(
            layouted.area,
            state.get(&self.state).as_str(),
            *state.get(&self.font_size),
            foreground_color,
            *state.get(&self.text_alignment),
            *state.get(&crate::theme::theme().text_box().vertical_alignment()),
        );
    }
}

pub struct DefaultHandler<P>(pub P);

impl<App, P> InputHandler<App> for DefaultHandler<P>
where
    P: rust_state::Path<App, String>,
{
    fn handle_character(&self, state: &Context<App>, character: char) {
        if character == '\x08' {
            state.update_value_with(self.0, move |current_text| {
                if !current_text.is_empty() {
                    current_text.pop();
                }
            });
        } else {
            state.update_value_with(self.0, move |current_text| {
                current_text.push(character);
            });
        }
    }
}
