use cgmath::{ Vector4, Vector2 };

use graphics::{ Renderer, Color };

pub struct RectangleComponent {
    position: Vector2<f32>,
    size: Vector2<f32>,
    corner_radius: Vector4<f32>,
    color: Color,
    focused_color: Color,
}

impl RectangleComponent {

    pub fn new(position: Vector2<f32>, size: Vector2<f32>, corner_radius: Vector4<f32>, color: Color, focused_color: Color) -> Self {
        return Self { position, size, corner_radius, color, focused_color };
    }

    pub fn render(&self, renderer: &mut Renderer, position: Vector2<f32>, focused: bool) {
        match focused {
            true => renderer.render_rectangle(self.position + position, self.size, self.corner_radius, self.focused_color),
            false => renderer.render_rectangle(self.position + position, self.size, self.corner_radius, self.color),
        }
    }
}
