use bytemuck::{Zeroable, Pod};
use cgmath::Vector3;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct WaterVertex {
    pub position: [f32; 3],
}

impl WaterVertex {
    pub fn new(position: Vector3<f32>) -> Self {
        Self { position: position.into() }
    }
}

vulkano::impl_vertex!(WaterVertex, position);
