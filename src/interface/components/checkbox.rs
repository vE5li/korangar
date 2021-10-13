use cgmath::Vector2;

use graphics::Color;

use super::super::*;

pub struct CheckboxComponent {
    offset: Vector2<f32>,
    size: Vector2<f32>,
    color: Color,
    state_key: StateKey,
}

impl CheckboxComponent {

    pub fn new(offset: Vector2<f32>, size: Vector2<f32>, color: Color, state_key: StateKey) -> Self {
        return Self { offset, size, color, state_key };
    }

    pub fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, position: Vector2<f32>) {
        let checked = state_provider.get(&self.state_key).to_boolean();
        renderer.render_checkbox(position + self.offset, self.size, self.color, checked);
    }
}
