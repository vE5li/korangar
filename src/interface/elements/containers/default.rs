use procedural::*;
use derive_new::new;
use num::Zero;

use crate::interface::traits::Element;
use crate::interface::types::*;
use crate::graphics::{Renderer, InterfaceRenderer};

#[derive(new)]
pub struct Container {
    elements: Vec<ElementCell>,
    size_constraint: SizeConstraint,
    #[new(value = "Size::zero()")]
    cached_size: Size,
    #[new(value = "Position::zero()")]
    cached_position: Position,
}

impl Container {

    pub const DEFAULT_SIZE: SizeConstraint = constraint!(100.0%, ?);
}

impl Element for Container {

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {

        let (mut size, position) = placement_resolver.allocate(&self.size_constraint);
        let mut inner_placement_resolver = placement_resolver.derive(Position::zero(), Size::zero());
        inner_placement_resolver.set_gaps(Size::new(5.0, 3.0));

        self.elements.iter_mut().for_each(|element| element.borrow_mut().resolve(&mut inner_placement_resolver, interface_settings, theme));

        if self.size_constraint.height.is_flexible() {
            let final_height = inner_placement_resolver.final_height();
            let final_height = self.size_constraint.validated_height(final_height, placement_resolver.get_avalible().y, placement_resolver.get_avalible().y, *interface_settings.scaling);
            size.y = Some(final_height);
            placement_resolver.register_height(final_height);
        }

        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.elements
            .iter_mut()
            .map(|element| element.borrow_mut().update())
            .fold(None, |current, other| current.zip_with(other, ChangeEvent::combine).or(current).or(other))
    }

    fn hovered_element(&self, mouse_position: Position) -> HoverInformation {
        let absolute_position = mouse_position - self.cached_position;

        if absolute_position.x >= 0.0 && absolute_position.y >= 0.0 && absolute_position.x <= self.cached_size.x && absolute_position.y <= self.cached_size.y {
            for element in &self.elements {
                match element.borrow().hovered_element(absolute_position) {
                    HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                    HoverInformation::Element(element) => return HoverInformation::Element(element),
                    HoverInformation::Ignored => return HoverInformation::Ignored,
                    HoverInformation::Missed => {},
                }
            }
        }

        HoverInformation::Missed
    }

    fn render(&self, render_target: &mut <InterfaceRenderer as Renderer>::Target, renderer: &InterfaceRenderer, state_provider: &StateProvider, interface_settings: &InterfaceSettings, theme: &Theme, parent_position: Position, clip_size: Size, hovered_element: Option<&dyn Element>, second_theme: bool) {
        let absolute_position = parent_position + self.cached_position;
        let clip_size = clip_size.zip(absolute_position + self.cached_size, f32::min);
        self.elements.iter().for_each(|element| element.borrow().render(render_target, renderer, state_provider, interface_settings, theme, absolute_position, clip_size, hovered_element, second_theme));
    }
}
