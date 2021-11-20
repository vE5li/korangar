mod text;
mod button;
mod checkbutton;
mod bar;

use cgmath::Vector2;

use graphics::Renderer;

use super::*;

pub use self::text::text;
pub use self::button::button;
pub use self::checkbutton::checkbutton;
pub use self::bar::bar;

macro_rules! get_component {
    ($element:expr, $name:ident) => ({
        for component in &$element.components {
            if let Component::$name(component) = component {
                return Some(component);
            }
        }
        None
    })
}

pub struct Element {
    components: Vec<Component>,
    element_index: usize,
    position: Vector2<f32>,
}

impl Element {

    pub fn new(components: Vec<Component>, element_index: usize, position: Vector2<f32>) -> Self {
        return Self { components, element_index, position };
    }

    fn relative_position(&self, interface_state: &InterfaceState) -> Vector2<f32> {
        match self.draggable() {
            Some(component) => return self.position + component.get_offset(interface_state),
            None => return self.position,
        };
    }

    pub fn hovered_element(&self, interface_state: &InterfaceState, mouse_position: Vector2<f32>) -> Option<&Element> {

        let hoverable = match self.hoverable() {
            Some(hoverable) => hoverable,
            None => return None,
        };

        let relative_mouse_position = mouse_position - self.relative_position(interface_state);

        if !hoverable.mouse_hovers(relative_mouse_position) {
            return None;
        }

        if let Some(container) = self.container() {
            if let Some(element) = container.hovered_element(interface_state, relative_mouse_position) {
                return Some(element);
            }
        };

        return Some(self);
    }

    pub fn index(&self) -> usize {
        return self.element_index;
    }

    pub fn hoverable(&self) -> Option<&HoverableComponent> {
        return get_component!(self, Hoverable);
    }

    pub fn clickable(&self) -> Option<&ClickableComponent> {
        return get_component!(self, Clickable);
    }

    pub fn draggable(&self) -> Option<&DraggableComponent> {
        return get_component!(self, Draggable);
    }

    fn container(&self) -> Option<&ContainerComponent> {
        return get_component!(self, Container);
    }

    pub fn render(&self, renderer: &mut Renderer, interface_state: &InterfaceState, state_provider: &StateProvider, parent_position: Vector2<f32>, hovered_index: usize) {

        let focused = hovered_index == self.element_index;
        let position = parent_position + self.relative_position(interface_state);

        for component in &self.components {
            match component {
                Component::Text(text) => text.render(renderer, position),
                Component::DynamicText(dynamic_text) => dynamic_text.render(renderer, state_provider, position),
                Component::Rectangle(rectangle) => rectangle.render(renderer, position, focused),
                Component::Stretch(stretch) => stretch.render(renderer, state_provider, position),
                Component::Checkbox(checkbox) => checkbox.render(renderer, state_provider, position),
                Component::Container(container) => container.render(renderer, interface_state, state_provider, position, hovered_index),
                _invisible => { },
            }
        }
    }
}
