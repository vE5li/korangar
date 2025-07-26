use std::cell::{RefCell, UnsafeCell};
use std::marker::PhantomData;

use rust_state::{Context, RustState, Selector};

use super::button::ButtonThemePathExt;
use crate::application::Application;
use crate::element::Element;
use crate::element::id::ElementIdGenerator;
use crate::element::store::{ElementStore, Persistent, PersistentData, PersistentExt};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{InputHandler, Layout, Resolver};
use crate::theme::{ThemePathGetter, theme};

#[derive(RustState)]
pub struct TextBoxTheme<App>
where
    App: Application,
{
    pub foreground_color: App::Color,
    pub background_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub hovered_background_color: App::Color,
    pub focused_foreground_color: App::Color,
    pub focused_background_color: App::Color,
    pub height: f32,
    pub corner_radius: App::CornerRadius,
    pub font_size: App::FontSize,
    pub text_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
}

pub struct TextBoxData {
    is_hidden: RefCell<bool>,
    hidden_text: UnsafeCell<String>,
}

impl PersistentData for TextBoxData {
    type Inputs = bool;

    fn new(inputs: Self::Inputs) -> Self {
        Self {
            is_hidden: RefCell::new(inputs),
            hidden_text: UnsafeCell::new(String::new()),
        }
    }
}

pub struct TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N> {
    pub text_marker: PhantomData<Text>,
    pub text: A,
    pub state: B,
    pub input_handler: C,
    pub hidable: D,
    pub foreground_color: E,
    pub background_color: F,
    pub hovered_foreground_color: G,
    pub hovered_background_color: H,
    pub focused_foreground_color: I,
    pub focused_background_color: J,
    pub height: K,
    pub corner_radius: L,
    pub font_size: M,
    pub text_alignment: N,
}

impl<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N> Persistent for TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N> {
    type Data = TextBoxData;
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N> Element<App> for TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N>
where
    App: Application,
    Text: AsRef<str> + 'static,
    A: Selector<App, Text>,
    B: Selector<App, String>,
    C: InputHandler<App> + 'static,
    D: Selector<App, bool>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, App::Color>,
    J: Selector<App, App::Color>,
    K: Selector<App, f32>,
    L: Selector<App, App::CornerRadius>,
    M: Selector<App, App::FontSize>,
    N: Selector<App, HorizontalAlignment>,
{
    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        _: &mut ElementStore,
        _: &mut ElementIdGenerator,
        resolver: &mut Resolver,
    ) -> Self::LayoutInfo {
        let height = state.get(&self.height);

        Self::LayoutInfo {
            area: resolver.with_height(*height),
        }
    }

    fn layout_element<'a>(
        &'a self,
        state: &'a Context<App>,
        store: &'a ElementStore,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, App>,
    ) {
        let hide_button = state.get(&self.hidable).then(|| {
            let button_size = layout_info.area.height - 6.0;
            let button_area = Area {
                left: layout_info.area.left + layout_info.area.width - button_size - 3.0,
                top: layout_info.area.top + 3.0,
                width: button_size,
                height: button_size,
            };

            let is_hoverered = layout.is_area_hovered_and_active(button_area);
            let persistent_data = self.get_persistent_data(store, true);

            if is_hoverered {
                layout.add_toggle(button_area, &persistent_data.is_hidden);
                layout.mark_hovered();
            }

            (button_area, is_hoverered, persistent_data)
        });

        let is_hovered = layout.is_area_hovered_and_active(layout_info.area);
        let is_focused = layout.is_element_focused(store.get_element_id());

        if is_hovered {
            layout.add_focus_area(layout_info.area, store.get_element_id());
            layout.mark_hovered();
        }

        if is_focused {
            layout.add_input_handler(&self.input_handler);
        }

        let background_color = match is_hovered {
            _ if is_focused => *state.get(&self.focused_background_color),
            true => *state.get(&self.hovered_background_color),
            false => *state.get(&self.background_color),
        };

        layout.add_rectangle(layout_info.area, *state.get(&self.corner_radius), background_color);

        let foreground_color = match is_hovered {
            _ if is_focused => *state.get(&self.focused_foreground_color),
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        let mut display_text = state.get(&self.state).as_str();

        if let Some((button_area, is_hovered, persistent_data)) = hide_button {
            let background_color = match is_hovered {
                true => *state.get(&theme().button().hovered_background_color()),
                false => *state.get(&theme().button().background_color()),
            };

            if *persistent_data.is_hidden.borrow() {
                // SAFETY:
                //
                // This is only used here to create a string with all '*' characters, so this
                // should be perfectly safe.
                let hidden_text = unsafe { &mut *persistent_data.hidden_text.get() };

                let display_text_length = display_text.len();
                if hidden_text.len() != display_text_length {
                    *hidden_text = "*".repeat(display_text_length);
                }

                display_text = hidden_text;
            }

            layout.add_rectangle(button_area, *state.get(&theme().button().corner_radius()), background_color);
        }

        // layout.add_text(
        //     area,
        //     state.get(&self.text).as_ref(),
        //     *state.get(&self.font_size),
        //     foreground_color,
        //     *state.get(&self.text_alignment),
        //     *state.get(&theme().text_box().vertical_alignment()),
        // );

        layout.add_text(
            layout_info.area,
            display_text,
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
