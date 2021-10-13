use cgmath::Vector2;

pub struct HoverableComponent {
    size: Vector2<f32>,
}

impl HoverableComponent {

    pub fn new(size: Vector2<f32>) -> Self {
        return Self { size };
    }

    pub fn mouse_hovers(&self, mouse_position: Vector2<f32>) -> bool {
        return mouse_position.x >= 0.0 && mouse_position.y >= 0.0 && mouse_position.x <= self.size.x && mouse_position.y <= self.size.y;
    }
}
