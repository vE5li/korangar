mod builder;

use std::fmt::Display;

use procedural::dimension_bound;

pub use self::builder::InputFieldBuilder;
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;

/// Local type alias to simplify the builder.
type EnterAction = Box<dyn FnMut() -> Vec<ClickAction>>;

pub struct InputField<TEXT: Display + 'static> {
    input_state: TrackedState<String>,
    ghost_text: TEXT,
    enter_action: EnterAction,
    length: usize,
    hidden: bool,
    width_bound: Option<DimensionBound>,
    state: ElementState,
}

impl<TEXT: Display + 'static> InputField<TEXT> {
    fn remove_character(&mut self) -> Vec<ClickAction> {
        self.input_state.with_mut(|input_state, changed| {
            if input_state.is_empty() {
                return Vec::new();
            }

            input_state.pop();
            changed();

            vec![ClickAction::ChangeEvent(ChangeEvent::RENDER_WINDOW)]
        })
    }

    fn add_character(&mut self, character: char) -> Vec<ClickAction> {
        self.input_state.with_mut(|input_state, changed| {
            if input_state.len() >= self.length {
                return Vec::new();
            }

            input_state.push(character);
            changed();

            vec![ClickAction::ChangeEvent(ChangeEvent::RENDER_WINDOW)]
        })
    }
}

impl<TEXT: Display + 'static> Element for InputField<TEXT> {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let size_bound = self
            .width_bound
            .as_ref()
            .unwrap_or(&dimension_bound!(100%))
            .add_height(theme.input.height_bound);

        self.state.resolve(placement_resolver, &size_bound);
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _update: &mut bool) -> Vec<ClickAction> {
        vec![ClickAction::FocusElement]
    }

    fn input_character(&mut self, character: char) -> Vec<ClickAction> {
        match character {
            '\u{8}' | '\u{7f}' => self.remove_character(),
            '\r' => (self.enter_action)(),
            character => self.add_character(character),
        }
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, screen_clip);

        let input_state = self.input_state.borrow();
        let is_hovererd = self.is_element_self(hovered_element);
        let is_focused = self.is_element_self(focused_element);
        let text_offset = theme.input.text_offset.get();

        let text = if input_state.is_empty() && !is_focused {
            self.ghost_text.to_string()
        } else if self.hidden {
            input_state.chars().map(|_| '*').collect()
        } else {
            input_state.clone()
        };

        let background_color = if is_hovererd {
            theme.input.hovered_background_color.get()
        } else if is_focused {
            theme.input.focused_background_color.get()
        } else {
            theme.input.background_color.get()
        };

        let text_color = if input_state.is_empty() && !is_focused {
            theme.input.ghost_text_color.get()
        } else if is_focused {
            theme.input.focused_text_color.get()
        } else {
            theme.input.text_color.get()
        };

        renderer.render_background(theme.input.corner_radius.get(), background_color);
        renderer.render_text(&text, text_offset, text_color, theme.input.font_size.get());

        if is_focused {
            let cursor_offset = (text_offset.left + theme.input.cursor_offset.get()) * interface_settings.scaling.get()
                + renderer.get_text_dimensions(&text, theme.input.font_size.get(), f32::MAX).x;

            let cursor_position = ScreenPosition::only_left(cursor_offset);
            let cursor_size = ScreenSize {
                width: theme.input.cursor_width.get(),
                height: self.state.cached_size.height,
            };

            renderer.render_rectangle(
                cursor_position,
                cursor_size,
                CornerRadius::default(),
                theme.input.text_color.get(),
            );
        }
    }
}
