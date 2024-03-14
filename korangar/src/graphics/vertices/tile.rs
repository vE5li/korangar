use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use vulkano::pipeline::graphics::vertex_input::Vertex;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod, Vertex)]
pub struct TileVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32_UINT)]
    pub identifier: u32,
}

impl TileVertex {
    pub const fn new(position: Vector3<f32>, identifier: u32) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            identifier,
        }
    }
}
