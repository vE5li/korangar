use cgmath::Vector2;

#[derive(Default, Debug, Clone, Copy)]
pub struct ScreenVertex {
    position: [f32; 2],
}

impl ScreenVertex {

    pub const fn new(position: Vector2<f32>) -> Self {
        return Self { position: [position.x, position.y] }
    }
}

vulkano::impl_vertex!(ScreenVertex, position);
