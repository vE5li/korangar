use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct TileVertex {
    pub position: [f32; 3],
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

vulkano::impl_vertex!(TileVertex, position, identifier);
