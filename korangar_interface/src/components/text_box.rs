use std::any::Any;
use std::cell::{RefCell, UnsafeCell};
use std::marker::PhantomData;

use rust_state::{Context, Path, RustState, Selector};

use crate::application::Application;
use crate::element::Element;
use crate::element::id::{ElementIdGenerator, FocusIdExt};
use crate::element::store::{ElementStore, Persistent, PersistentData, PersistentExt};
use crate::event::{ClickAction, Event, EventQueue};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Icon, InputHandler, Layout, Resolver};
use crate::theme::ThemePathGetter;

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
    pub hide_icon_color: App::Color,
    pub hovered_hide_icon_color: App::Color,
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

pub struct TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Id> {
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
    pub hide_icon_color: K,
    pub hovered_hide_icon_color: L,
    pub height: M,
    pub corner_radius: N,
    pub font_size: O,
    pub text_alignment: P,
    pub focus_id: Id,
}

impl<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Id> Persistent
    for TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Id>
{
    type Data = TextBoxData;
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Id> Element<App>
    for TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Id>
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
    K: Selector<App, App::Color>,
    L: Selector<App, App::Color>,
    M: Selector<App, f32>,
    N: Selector<App, App::CornerRadius>,
    O: Selector<App, App::FontSize>,
    P: Selector<App, HorizontalAlignment>,
    Id: Any,
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
        let element_id = store.get_element_id();
        let is_focused = layout.is_element_focused(element_id);

        let hide_button = state.get(&self.hidable).then(|| {
            let button_area = Area {
                left: layout_info.area.left + layout_info.area.width - layout_info.area.height - layout_info.area.height / 2.0,
                top: layout_info.area.top,
                width: layout_info.area.height + layout_info.area.height / 2.0,
                height: layout_info.area.height,
            };

            let is_hoverered = layout.is_area_hovered_and_active(button_area);
            let persistent_data = self.get_persistent_data(store, true);

            if is_hoverered {
                layout.add_toggle(button_area, &persistent_data.is_hidden);
                layout.mark_hovered();

                // If the text field is already focused, we don't want to unfocus it when
                // clicking the hide button. So we add another focus area to
                // re-focus if the button is clicked.
                if is_focused {
                    layout.add_focus_area(button_area, element_id);
                }
            }

            (button_area, is_hoverered, persistent_data)
        });

        let is_hovered = layout.is_area_hovered_and_active(layout_info.area);

        if is_hovered {
            layout.add_focus_area(layout_info.area, element_id);
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

        let corner_radius = *state.get(&self.corner_radius);

        layout.add_rectangle(layout_info.area, corner_radius, background_color);

        let foreground_color = match is_hovered {
            _ if is_focused => *state.get(&self.focused_foreground_color),
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        let mut display_text = state.get(&self.state).as_str();

        if let Some((button_area, is_hovered, persistent_data)) = hide_button {
            let is_hidden = *persistent_data.is_hidden.borrow();

            if is_hidden {
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

            let icon_area = Area {
                left: button_area.left + 4.0 + layout_info.area.height / 4.0,
                top: button_area.top + 4.0,
                width: button_area.height - 8.0,
                height: button_area.height - 8.0,
            };
            let icon_color = match is_hovered {
                true => *state.get(&self.hovered_hide_icon_color),
                false => *state.get(&self.hide_icon_color),
            };

            layout.add_icon(icon_area, Icon::Eye { open: is_hidden }, icon_color);
        }

        // layout.add_text(
        //     area,
        //     state.get(&self.text).as_ref(),
        //     *state.get(&self.font_size),
        //     foreground_color,
        //     *state.get(&self.text_alignment),
        //     *state.get(&theme().text_box().vertical_alignment()),
        // );

        layout.register_focus_id(self.focus_id.focus_id(), element_id);

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

pub struct DefaultHandler<P, A, const INPUT_LENGTH: usize> {
    path: P,
    action: A,
}

impl<P, A, const INPUT_LENGTH: usize> DefaultHandler<P, A, INPUT_LENGTH> {
    pub fn new(path: P, action: A) -> Self {
        Self { path, action }
    }
}

impl<App, P, A, const INPUT_LENGTH: usize> InputHandler<App> for DefaultHandler<P, A, INPUT_LENGTH>
where
    App: Application,
    P: Path<App, String>,
    A: ClickAction<App>,
{
    fn handle_character(&self, state: &Context<App>, queue: &mut EventQueue<App>, character: char) {
        if character == '\x09' || character == '\x0d' {
            // On tab or enter
            self.action.execute(state, queue);
        } else if character == '\x1b' {
            // On escape
            queue.queue(Event::Unfocus);
        } else if character == '\x08' {
            state.update_value_with(self.path, move |current_text| {
                if !current_text.is_empty() {
                    current_text.pop();
                }
            });
        } else if !character.is_control() {
            state.update_value_with(self.path, move |current_text| {
                if current_text.len() < INPUT_LENGTH {
                    current_text.push(character);
                }
            });
        }
    }
}
