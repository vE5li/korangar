use cgmath::Vector2;

#[derive(Default, Debug, Clone, Copy)]
pub struct ScreenVertex {
    pub position: [f32; 2],
}

impl ScreenVertex {

    pub const fn new(position: Vector2<f32>) -> Self { // replace with derive new when const fn becomes an option
        return Self { position: [position.x, position.y] }
    }
}

vulkano::impl_vertex!(ScreenVertex, position);
