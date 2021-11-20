use cgmath::{ Vector4, Vector2 };

use graphics::Color;

use super::super::*;

pub struct StretchComponent {
    offset: Vector2<f32>,
    size: Vector2<f32>,
    corner_radius: Vector4<f32>,
    color: Color,
    maximum_value_key: StateKey,
    current_value_key: StateKey,
}

impl StretchComponent {

    pub fn new(offset: Vector2<f32>, size: Vector2<f32>, corner_radius: Vector4<f32>, color: Color, maximum_value_key: StateKey, current_value_key: StateKey) -> Self {
        return Self { offset, size, corner_radius, color, maximum_value_key, current_value_key };
    }

    pub fn render(&self, renderer: &mut Renderer, state_provider: &StateProvider, position: Vector2<f32>) {

        let maximum = state_provider.get(&self.maximum_value_key).to_number() as f32;
        let current = state_provider.get(&self.current_value_key).to_number() as f32;
        let size = Vector2::new((self.size.x / maximum) * current, self.size.y);

        renderer.render_rectangle(position + self.offset, size, self.corner_radius, self.color);
    }
}
