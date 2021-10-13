use cgmath::Vector2;

use super::super::*;

pub struct DraggableComponent {
    identifier: usize,
}

impl DraggableComponent {

    pub fn new(interface_state: &mut InterfaceState) -> Self {
        let identifier = interface_state.register_draggable();
        return Self { identifier };
    }

    pub fn get_identifier(&self) -> usize {
        return self.identifier;
    }

    pub fn get_offset(&self, interface_state: &InterfaceState) -> Vector2<f32> {
        return interface_state.get_offset(self.identifier);
    }
}
