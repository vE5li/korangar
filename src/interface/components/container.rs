use cgmath::Vector2;

use super::super::*;

pub struct ContainerComponent {
    elements: Vec<Element>,
}

impl ContainerComponent {

    pub fn new(elements: Vec<Element>) -> Self {
        return Self { elements };
    }

    pub fn hovered_element(&self, interface_state: &InterfaceState, mouse_position: Vector2<f32>) -> Option<&Element> {

        for element in &self.elements {
            if let Some(element) = element.hovered_element(interface_state, mouse_position) {
                return Some(element);
            }
        }

        return None;
    }

    pub fn render(&self, renderer: &mut Renderer, interface_state: &InterfaceState, state_provider: &StateProvider, position: Vector2<f32>, hovered_index: usize) {
        self.elements.iter().for_each(|element| element.render(renderer, interface_state, state_provider, position, hovered_index));
    }
}
