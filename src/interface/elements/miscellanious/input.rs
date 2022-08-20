use derive_new::new;
use num::Zero;
use std::cell::RefCell;
use std::rc::Rc;

use crate::types::maths::*;
use crate::interface::traits::Element;
use crate::interface::types::*;
use crate::graphics::{ Renderer, InterfaceRenderer };

#[derive(new)]
pub struct InputField<const LENGTH: usize, const HIDDEN: bool> {
    display: Rc<RefCell<String>>,
    ghost_text: &'static str,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl<const LENGTH: usize, const HIDDEN: bool> InputField<LENGTH, HIDDEN> {

    fn remove_character(&mut self) -> Option<ChangeEvent> {

        let mut display = self.display.borrow_mut();

        if display.is_empty() {
            return None;
        }

        display.pop();
        Some(ChangeEvent::RerenderWindow)
    }

    fn add_character(&mut self, character: char) -> Option<ChangeEvent> {

        let mut display = self.display.borrow_mut();

        if display.len() >= LENGTH {
            return None;
        }

        display.push(character);
        Some(ChangeEvent::RerenderWindow)
    }
}

impl<const LENGTH: usize, const HIDDEN: bool> Element for InputField<LENGTH, HIDDEN> {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &Theme) {
        let (size, position) = placement_resolver.allocate(&theme.input.size_constraint);
        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        let absolute_position = mouse_position - self.cached_position;

        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.cached_size.x && absolute_position.y <= self.cached_size.y {
            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    fn left_click(&mut self, _update: &mut bool) -> Option<ClickAction> {
        Some(ClickAction::FocusElement)
    }

    fn input_character(&mut self, character: char) -> Option<ChangeEvent> {
        match character {
            '\u{8}' => self.remove_character(),
            character => self.add_character(character),
        }
    }

    fn render(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, renderer: &InterfaceRenderer, _state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, hovered_element: Option<&dyn Element>, focused_element: Option<&dyn Element>, _second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);

        let display: &String = &RefCell::borrow(&self.display);
        let is_hovererd = matches!(hovered_element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ()));
        let is_focused = matches!(focused_element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ()));

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

        renderer.render_rectangle(render_target, absolute_position, self.cached_size, clip_size, *theme.input.border_radius * *interface_settings.scaling, background_color);
        renderer.render_text(render_target, &text, absolute_position, clip_size, text_color, *theme.input.font_size * *interface_settings.scaling);

        if is_focused {
            let cursor_offset = *theme.input.cursor_offset * *interface_settings.scaling;
            renderer.render_rectangle(render_target, absolute_position + Vector2::new(cursor_offset + text.len() as f32 * *theme.input.font_size * *interface_settings.scaling * 0.5, 0.0), Vector2::new(*theme.input.cursor_width, self.cached_size.y), clip_size, Vector4::from_value(0.0), *theme.input.text_color);
        }
    }
}
