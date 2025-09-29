use std::any::Any;
use std::cell::{Cell, UnsafeCell};
use std::marker::PhantomData;

use rust_state::{Context, Path, RustState, Selector};

use crate::application::{Application, Size};
use crate::element::Element;
use crate::element::id::{ElementId, FocusIdExt};
use crate::element::store::{ElementStore, ElementStoreMut, Persistent, PersistentData, PersistentExt};
use crate::event::{ClickHandler, Event, EventQueue, InputHandler};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{Icon, MouseButton, Resolver, WindowLayout};

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextBoxTheme<App>
where
    App: Application,
{
    pub foreground_color: App::Color,
    pub background_color: App::Color,
    pub highlight_color: App::Color,
    pub hovered_foreground_color: App::Color,
    pub hovered_background_color: App::Color,
    pub focused_foreground_color: App::Color,
    pub focused_background_color: App::Color,
    pub ghost_foreground_color: App::Color,
    pub hide_icon_color: App::Color,
    pub hovered_hide_icon_color: App::Color,
    pub shadow_color: App::Color,
    pub shadow_padding: App::ShadowPadding,
    pub height: f32,
    pub corner_diameter: App::CornerDiameter,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

pub struct TextBoxData {
    is_hidden: Cell<bool>,
    hidden_text: UnsafeCell<String>,
}

impl PersistentData for TextBoxData {
    type Inputs = bool;

    fn from_inputs(inputs: Self::Inputs) -> Self {
        Self {
            is_hidden: Cell::new(inputs),
            hidden_text: UnsafeCell::new(String::new()),
        }
    }
}

impl<App> ClickHandler<App> for TextBoxData
where
    App: Application,
{
    fn handle_click(&self, _: &Context<App>, _: &mut EventQueue<App>) {
        let is_hidden = self.is_hidden.get();
        self.is_hidden.set(!is_hidden);
    }
}

#[derive(Default)]
struct FocusClick {
    element_id: Option<ElementId>,
}

impl FocusClick {
    fn update(&mut self, element_id: ElementId) {
        self.element_id = Some(element_id);
    }
}

impl<App> ClickHandler<App> for FocusClick
where
    App: Application,
{
    fn handle_click(&self, _: &Context<App>, queue: &mut EventQueue<App>) {
        let element_id = *self.element_id.as_ref().unwrap();
        queue.queue(Event::FocusElementPost { element_id });
    }
}

pub struct TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, Id> {
    text_marker: PhantomData<Text>,
    ghost_text: A,
    state: B,
    input_handler: C,
    hidable: D,
    foreground_color: E,
    background_color: F,
    highlight_color: G,
    hovered_foreground_color: H,
    hovered_background_color: I,
    focused_foreground_color: J,
    focused_background_color: K,
    ghost_foreground_color: L,
    hide_icon_color: M,
    hovered_hide_icon_color: N,
    shadow_color: O,
    shadow_padding: P,
    height: Q,
    corner_diameter: R,
    font_size: S,
    horizontal_alignment: T,
    vertical_alignment: U,
    overflow_behavior: V,
    focus_id: Id,
    focus_click: FocusClick,
}

impl<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, Id>
    TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, Id>
{
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        ghost_text: A,
        state: B,
        input_handler: C,
        hidable: D,
        foreground_color: E,
        background_color: F,
        highlight_color: G,
        hovered_foreground_color: H,
        hovered_background_color: I,
        focused_foreground_color: J,
        focused_background_color: K,
        ghost_foreground_color: L,
        hide_icon_color: M,
        hovered_hide_icon_color: N,
        shadow_color: O,
        shadow_padding: P,
        height: Q,
        corner_diameter: R,
        font_size: S,
        horizontal_alignment: T,
        vertical_alignment: U,
        overflow_behavior: V,
        focus_id: Id,
    ) -> Self {
        Self {
            text_marker: PhantomData,
            ghost_text,
            state,
            input_handler,
            hidable,
            foreground_color,
            background_color,
            highlight_color,
            hovered_foreground_color,
            hovered_background_color,
            focused_foreground_color,
            focused_background_color,
            ghost_foreground_color,
            hide_icon_color,
            hovered_hide_icon_color,
            shadow_color,
            shadow_padding,
            height,
            corner_diameter,
            font_size,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
            focus_id,
            focus_click: FocusClick::default(),
        }
    }
}

impl<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, Id> Persistent
    for TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, Id>
{
    type Data = TextBoxData;
}

impl<App, Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, Id> Element<App>
    for TextBox<Text, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, Id>
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
    M: Selector<App, App::Color>,
    N: Selector<App, App::Color>,
    O: Selector<App, App::Color>,
    P: Selector<App, App::ShadowPadding>,
    Q: Selector<App, f32>,
    R: Selector<App, App::CornerDiameter>,
    S: Selector<App, App::FontSize>,
    T: Selector<App, HorizontalAlignment>,
    U: Selector<App, VerticalAlignment>,
    V: Selector<App, App::OverflowBehavior>,
    Id: Any,
{
    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo {
        let height = *state.get(&self.height);

        let mut display_text = state.get(&self.state).as_str();

        self.focus_click.update(store.get_element_id());

        if *state.get(&self.hidable) {
            let persistent_data = self.get_persistent_data(&store, true);
            let is_hidden = persistent_data.is_hidden.get();

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
        }

        if display_text.is_empty() {
            display_text = state.get(&self.ghost_text).as_ref();
        }

        let (size, font_size) = resolver.get_text_dimensions(
            display_text,
            *state.get(&self.foreground_color),
            *state.get(&self.highlight_color),
            *state.get(&self.font_size),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.overflow_behavior),
        );

        Self::LayoutInfo {
            area: resolver.with_height(height.max(size.height())),
            font_size,
        }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
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

            let is_hoverered = button_area.check().run(layout);
            let persistent_data = self.get_persistent_data(&store, true);

            if is_hoverered {
                layout.register_click_handler(MouseButton::Left, persistent_data);

                // If the text field is already focused, we don't want to unfocus it when
                // clicking the hide button. So we add another click area to
                // re-focus if the button is clicked.
                if is_focused {
                    layout.register_click_handler(MouseButton::Left, &self.focus_click);
                }
            }

            (button_area, is_hoverered, persistent_data)
        });

        let is_hovered = layout_info.area.check().run(layout);

        if is_hovered {
            layout.register_click_handler(MouseButton::Left, &self.focus_click);
        }

        if is_focused {
            layout.register_input_handler(&self.input_handler);
        }

        let background_color = match is_hovered {
            _ if is_focused => *state.get(&self.focused_background_color),
            true => *state.get(&self.hovered_background_color),
            false => *state.get(&self.background_color),
        };

        let corner_diameter = *state.get(&self.corner_diameter);

        layout.add_rectangle(
            layout_info.area,
            corner_diameter,
            background_color,
            *state.get(&self.shadow_color),
            *state.get(&self.shadow_padding),
        );

        let mut display_text = state.get(&self.state).as_str();

        if let Some((button_area, is_hovered, persistent_data)) = hide_button {
            let is_hidden = persistent_data.is_hidden.get();

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

        let show_ghost_text = display_text.is_empty() && !is_focused;

        if show_ghost_text {
            display_text = state.get(&self.ghost_text).as_ref();
        }

        let foreground_color = match is_hovered {
            _ if show_ghost_text => *state.get(&self.ghost_foreground_color),
            _ if is_focused => *state.get(&self.focused_foreground_color),
            true => *state.get(&self.hovered_foreground_color),
            false => *state.get(&self.foreground_color),
        };

        layout.add_text(
            layout_info.area,
            display_text,
            layout_info.font_size,
            foreground_color,
            *state.get(&self.highlight_color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );

        layout.register_focus_id(self.focus_id.focus_id(), element_id);
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
    A: ClickHandler<App>,
{
    fn handle_character(&self, state: &Context<App>, queue: &mut EventQueue<App>, character: char) {
        if character == '\x09' || character == '\x0d' {
            // On tab or enter
            self.action.handle_click(state, queue);
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
