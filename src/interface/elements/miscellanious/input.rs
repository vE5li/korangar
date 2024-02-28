use std::cell::RefCell;
use std::rc::Rc;

use derive_new::new;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

#[derive(new)]
pub struct InputField<const LENGTH: usize, const HIDDEN: bool = false> {
    display: Rc<RefCell<String>>,
    ghost_text: &'static str,
    action: Box<dyn Fn() -> Vec<ClickAction>>,
    width_constraint: DimensionConstraint,
    #[new(default)]
    state: ElementState,
}

impl<const LENGTH: usize, const HIDDEN: bool> InputField<LENGTH, HIDDEN> {
    fn remove_character(&mut self) -> Vec<ClickAction> {
        let mut display = self.display.borrow_mut();

        if display.is_empty() {
            return Vec::new();
        }

        display.pop();
        vec![ClickAction::ChangeEvent(ChangeEvent::RENDER_WINDOW)]
    }

    fn add_character(&mut self, character: char) -> Vec<ClickAction> {
        let mut display = self.display.borrow_mut();

        if display.len() >= LENGTH {
            return Vec::new();
        }

        display.push(character);
        vec![ClickAction::ChangeEvent(ChangeEvent::RENDER_WINDOW)]
    }
}

impl<const LENGTH: usize, const HIDDEN: bool> Element for InputField<LENGTH, HIDDEN> {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let size_constraint = self.width_constraint.add_height(theme.input.height_constraint);
        self.state.resolve(placement_resolver, &size_constraint);
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
            '\r' => (self.action)(),
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

        let display: &String = &RefCell::borrow(&self.display);
        let is_hovererd = self.is_element_self(hovered_element);
        let is_focused = self.is_element_self(focused_element);
        let text_offset = *theme.input.text_offset * *interface_settings.scaling;

        let text = if display.is_empty() && !is_focused {
            self.ghost_text.to_string()
        } else if HIDDEN {
            display.chars().map(|_| '*').collect()
        } else {
            display.clone()
        };

        let background_color = if is_hovererd {
            *theme.input.hovered_background_color
        } else if is_focused {
            *theme.input.focused_background_color
        } else {
            *theme.input.background_color
        };

        let text_color = if display.is_empty() && !is_focused {
            *theme.input.ghost_text_color
        } else if is_focused {
            *theme.input.focused_text_color
        } else {
            *theme.input.text_color
        };

        let text_position = ScreenPosition {
            left: text_offset.x,
            top: text_offset.y,
        };

        renderer.render_background((*theme.input.corner_radius).into(), background_color);
        renderer.render_text(&text, text_position, text_color, *theme.input.font_size);

        if is_focused {
            let cursor_offset = text_offset.x
                + *theme.input.cursor_offset * *interface_settings.scaling
                + renderer.get_text_dimensions(&text, *theme.input.font_size, f32::MAX).x;

            let cursor_position = ScreenPosition {
                left: cursor_offset,
                top: 0.0,
            };

            let cursor_size = ScreenSize {
                width: *theme.input.cursor_width,
                height: self.state.cached_size.height,
            };

            renderer.render_rectangle(
                cursor_position,
                cursor_size,
                CornerRadius::uniform(0.0),
                *theme.input.text_color,
            );
        }
    }
}
