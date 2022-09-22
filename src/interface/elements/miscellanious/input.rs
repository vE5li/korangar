use std::cell::RefCell;
use std::rc::Rc;

use cgmath::{Array, Vector2, Vector4};
use derive_new::new;
use num::Zero;

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::interface::{Element, *};

#[derive(new)]
pub struct InputField<const LENGTH: usize, const HIDDEN: bool = false> {
    display: Rc<RefCell<String>>,
    ghost_text: &'static str,
    action: Box<dyn Fn() -> Option<ClickAction>>,
    width_constraint: DimensionConstraint,
    #[new(default)]
    state: ElementState,
}

impl<const LENGTH: usize, const HIDDEN: bool> InputField<LENGTH, HIDDEN> {

    fn remove_character(&mut self) -> Option<ClickAction> {

        let mut display = self.display.borrow_mut();

        if display.is_empty() {
            return None;
        }

        display.pop();
        Some(ClickAction::ChangeEvent(ChangeEvent::RerenderWindow))
    }

    fn add_character(&mut self, character: char) -> Option<ClickAction> {

        let mut display = self.display.borrow_mut();

        if display.len() >= LENGTH {
            return None;
        }

        display.push(character);
        Some(ClickAction::ChangeEvent(ChangeEvent::RerenderWindow))
    }
}

impl<const LENGTH: usize, const HIDDEN: bool> Element for InputField<LENGTH, HIDDEN> {

    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {

        let size_constraint = self.width_constraint.add_height(theme.input.height_constraint);
        self.state.resolve(placement_resolver, &size_constraint);
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        self.state.hovered_element(mouse_position)
    }

    fn left_click(&mut self, _update: &mut bool) -> Option<ClickAction> {
        Some(ClickAction::FocusElement)
    }

    fn input_character(&mut self, character: char) -> Option<ClickAction> {
        match character {
            '\u{8}' => self.remove_character(),
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
        theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        _second_theme: bool,
    ) {

        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let display: &String = &RefCell::borrow(&self.display);
        let is_hovererd = self.is_element_self(hovered_element);
        let is_focused = self.is_element_self(focused_element);

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

        renderer.render_background(*theme.input.border_radius, background_color);

        renderer.render_text(&text, Vector2::zero(), text_color, *theme.input.font_size);

        if is_focused {

            let cursor_offset = *theme.input.cursor_offset * *interface_settings.scaling;

            renderer.render_rectangle(
                Vector2::new(
                    cursor_offset + text.len() as f32 * *theme.input.font_size * *interface_settings.scaling * 0.5,
                    0.0,
                ),
                Vector2::new(*theme.input.cursor_width, self.state.cached_size.y),
                Vector4::from_value(0.0),
                *theme.input.text_color,
            );
        }
    }
}
