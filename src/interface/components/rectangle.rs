use cgmath::Vector2;

use graphics::{ Renderer, Color };

pub struct RectangleComponent {
    size: Vector2<f32>,
    color: Color,
    focused_color: Color,
}

impl RectangleComponent {

    pub fn new(size: Vector2<f32>, color: Color, focused_color: Color) -> Self {
        return Self { size, color, focused_color };
    }

    pub fn render(&self, renderer: &mut Renderer, position: Vector2<f32>, focused: bool) {
        match focused {
            true => renderer.render_background(position, self.size, self.focused_color),
            false => renderer.render_background(position, self.size, self.color),
        }
    }
}
