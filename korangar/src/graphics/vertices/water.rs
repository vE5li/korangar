use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use vulkano::pipeline::graphics::vertex_input::Vertex;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod, Vertex)]
pub struct WaterVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
}

impl WaterVertex {
    pub fn new(position: Vector3<f32>) -> Self {
        Self { position: position.into() }
    }
}
