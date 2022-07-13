use std::rc::Rc;

use crate::types::maths::Vector2;
use crate::graphics::Renderer;
use crate::interface::traits::{ Element, Window };
use crate::interface::types::*;
use crate::interface::elements::{ DragButton, CloseButton };
use crate::interface::{ StateProvider, WindowCache, ElementCell, SizeConstraint, Size, PartialSize, Position };

pub struct FramedWindow {
    window_class: Option<String>,
    position: Vector2<f32>,
    size_constraint: SizeConstraint,
    size: Vector2<f32>,
    elements: Vec<ElementCell>,
}

impl FramedWindow {

    pub fn new(window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size, window_title: String, window_class: Option<String>, mut elements: Vec<ElementCell>, size_constraint: SizeConstraint) -> Self {

        let drag_button = cell!(DragButton::new(window_title));
        let close_button = cell!(CloseButton::new());
        elements.insert(0, close_button);
        elements.insert(0, drag_button);

        let (cached_position, cached_size) = window_class
            .as_ref()
            .and_then(|window_class| window_cache.get_window_state(window_class))
            .unzip();

        let size = cached_size
            .map(|size| size_constraint.validated_size(size, avalible_space, *interface_settings.scaling))
            .unwrap_or_else(|| size_constraint.resolve(avalible_space, avalible_space, *interface_settings.scaling).finalize_or(0.0));

        let position = cached_position
            .map(|position| size_constraint.validated_position(position, size, avalible_space))
            .unwrap_or((avalible_space - size) / 2.0);

        Self {
            window_class,
            position,
            size_constraint,
            size,
            elements,
        }
    }
}

impl Window for FramedWindow {

    fn get_window_class(&self) -> Option<&str> {
        self.window_class.as_ref().map(|window_class| window_class.as_str())
    }

    fn has_transparency(&self, theme: &Theme) -> bool {
        theme.window.background_color.alpha != 255
    }

    fn resolve(&mut self, interface_settings: &InterfaceSettings, theme: &Theme, avalible_space: Size) -> (Option<&str>, Vector2<f32>, Size) {

        let height = match self.size_constraint.height.is_flexible() {
            true => None,
            false => Some(self.size.y),
        };
        let mut placement_resolver = PlacementResolver::new(PartialSize::new(self.size.x, height), Vector2::new(0.0, 0.0), *theme.window.border_size, *theme.window.gaps, *interface_settings.scaling);

        self.elements.iter_mut().for_each(|element| element.borrow_mut().resolve(&mut placement_resolver, interface_settings, theme));

        if self.size_constraint.height.is_flexible() {
            let final_height = theme.window.border_size.y + placement_resolver.final_height();
            let final_height = self.size_constraint.validated_height(final_height, avalible_space.y.into(), avalible_space.y.into(), *interface_settings.scaling);
            self.size.y = final_height;
            self.validate_size(interface_settings, avalible_space);
        }

        self.validate_position(avalible_space);

        (self.window_class.as_deref(), self.position, self.size)
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.elements
            .iter_mut()
            .map(|element| element.borrow_mut().update())
            .fold(None, |current, other| current.zip_with(other, ChangeEvent::combine).or(current).or(other))
    }

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> HoverInformation {
        let absolute_position = mouse_position - self.position;

        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.size.x && absolute_position.y <= self.size.y {

            for element in &self.elements {
                match element.borrow().hovered_element(absolute_position) {
                    HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                    HoverInformation::Element(element) => return HoverInformation::Element(element),
                    HoverInformation::Ignored => return HoverInformation::Ignored,
                    HoverInformation::Missed => {},
                }
            }

            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    fn get_area(&self) -> (Position, Size) {
        (self.position, self.size)
    }

    fn hovers_area(&self, position: Position, size: Size) -> bool {

        let self_combined = self.position + self.size;
        let area_combined = position + size;

        self_combined.x > position.x && self.position.x < area_combined.x && self_combined.y > position.y && self.position.y < area_combined.y
    }

    fn offset(&mut self, avalible_space: Size, offset: Position) -> Option<(&str, Position)> {
        self.position += offset;
        self.validate_position(avalible_space);
        self.window_class.as_ref().map(|window_class| (window_class.as_str(), self.position))
    }

    fn validate_position(&mut self, avalible_space: Size) {
        self.position = self.size_constraint.validated_position(self.position, self.size, avalible_space);
    }

    fn resize(&mut self, interface_settings: &InterfaceSettings, _theme: &Theme, avalible_space: Size, growth: Size) -> (Option<&str>, Size) {
        self.size += growth;
        self.validate_size(interface_settings, avalible_space);
        (self.window_class.as_deref(), self.size)
    }

    fn validate_size(&mut self, interface_settings: &InterfaceSettings, avalible_space: Size) {
        self.size = self.size_constraint.validated_size(self.size, avalible_space, *interface_settings.scaling);
    }

    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, hovered_element: Option<&dyn Element>) {
        renderer.render_rectangle(self.position, self.size, self.position + self.size, *theme.window.border_radius, *theme.window.background_color);
        self.elements.iter().for_each(|element| element.borrow().render(renderer, state_provider, interface_settings, theme, self.position, self.position + self.size, hovered_element, false));
    }
}
