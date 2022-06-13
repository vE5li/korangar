use std::rc::Rc;

use types::maths::Vector2;
use graphics::Renderer;
use interface::traits::{ Element, Window };
use interface::types::{ PlacementResolver, Theme, HoverInformation, InterfaceSettings };
use interface::elements::{ DragButton, CloseButton };
use interface::{ StateProvider, WindowCache, ElementCell, SizeConstraint, Size, PartialSize, Position };

pub struct FramedWindow {
    identifier: Option<String>,
    position: Vector2<f32>,
    size_constraint: SizeConstraint,
    size: Vector2<f32>,
    elements: Vec<ElementCell>,
}

impl FramedWindow {

    pub fn new(window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size, window_title: String, identifier: Option<String>, mut elements: Vec<ElementCell>, size_constraint: SizeConstraint) -> Self {

        let drag_button = cell!(DragButton::new(window_title));
        let close_button = cell!(CloseButton::new());
        elements.insert(0, close_button);
        elements.insert(0, drag_button);

        let (position, size) = if let Some(identifier) = &identifier {
            let (cached_position, cached_size) = window_cache.get_window_state(identifier.as_str()).unzip();

            let position = cached_position
                // validate position
                .unwrap_or(Vector2::new(100.0, 300.0));

            let size = cached_size
                .map(|size| size_constraint.validated_size(size, avalible_space, *interface_settings.scaling))
                .unwrap_or(size_constraint.resolve(avalible_space, avalible_space, *interface_settings.scaling).finalize_or(0.0));

            (position, size)
        } else {
            let position = Vector2::new(100.0, 300.0);
            let size = size_constraint.resolve(avalible_space, avalible_space, *interface_settings.scaling).finalize_or(0.0);
            (position, size)
        };

        Self {
            identifier,
            position,
            size_constraint,
            size,
            elements,
        }
    }
}

impl Window for FramedWindow {

    fn identifier_matches(&self, other_identifier: &str) -> bool {
        self.identifier.as_ref().map_or(false, |identifier| identifier == other_identifier)
    }

    fn update(&mut self, interface_settings: &InterfaceSettings, theme: &Theme, avalible_space: Size) -> (Option<&str>, Vector2<f32>, Size) {

        let height = match self.size_constraint.height.is_flexible() {
            true => None,
            false => Some(self.size.y),
        };
        let mut placement_resolver = PlacementResolver::new(PartialSize::new(self.size.x, height), Vector2::new(0.0, 0.0), *theme.window.border_size, *theme.window.gaps, *interface_settings.scaling);

        self.elements.iter_mut().for_each(|element| element.borrow_mut().update(&mut placement_resolver, interface_settings, theme));

        if self.size_constraint.height.is_flexible() {
            let final_height = theme.window.border_size.y + placement_resolver.final_height();
            let final_height = self.size_constraint.validated_height(final_height, avalible_space.y.into(), avalible_space.y.into(), *interface_settings.scaling);
            self.size.y = final_height;
            self.validate_size(interface_settings, avalible_space);
        }

        (self.identifier.as_ref().map(|identifier| identifier.as_str()), self.position, self.size)
    }

    fn hovered_element(&self, mouse_position: Vector2<f32>) -> HoverInformation {
        let absolute_position = mouse_position - self.position;

        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.size.x && absolute_position.y <= self.size.y {

            for element in &self.elements {
                match element.borrow().hovered_element(absolute_position) {
                    HoverInformation::Hovered => return HoverInformation::Element(Rc::clone(element)),
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

    fn offset(&mut self, offset: Position) -> Option<(&str, Position)> {
        self.position += offset;
        self.identifier.as_ref().map(|identifier| (identifier.as_str(), self.position))
    }

    fn resize(&mut self, interface_settings: &InterfaceSettings, theme: &Theme, avalible_space: Size, growth: Size) -> (Option<&str>, Size) {
        self.size += growth;
        self.validate_size(interface_settings, avalible_space);
        self.update(interface_settings, theme, avalible_space);
        (self.identifier.as_ref().map(|identifier| identifier.as_str()), self.size)
    }

    fn validate_size(&mut self, interface_settings: &InterfaceSettings, avalible_space: Size) {
        self.size = self.size_constraint.validated_size(self.size, avalible_space, *interface_settings.scaling);
    }

    fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, hovered_element: Option<&dyn Element>) {
        renderer.render_rectangle(self.position, self.size, self.position + self.size, *theme.window.border_radius, theme.window.background_color);
        self.elements.iter().for_each(|element| element.borrow().render(renderer, state_provider, interface_settings, theme, self.position, self.position + self.size, hovered_element, false));
    }
}
