use bytemuck::{Pod, Zeroable};
use cgmath::Vector2;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct ScreenVertex {
    pub position: [f32; 2],
}

impl ScreenVertex {
    pub const fn new(position: Vector2<f32>) -> Self {
        // replace with derive new when const fn becomes an option
        Self {
            position: [position.x, position.y],
        }
    }
}

vulkano::impl_vertex!(ScreenVertex, position);
