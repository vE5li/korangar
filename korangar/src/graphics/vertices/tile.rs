use bytemuck::{Pod, Zeroable};
use cgmath::Point3;
use wgpu::{vertex_attr_array, VertexAttribute, VertexBufferLayout, VertexStepMode};

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct TileVertex {
    pub position: [f32; 3],
    pub identifier: u32,
}

impl TileVertex {
    pub const fn new(position: Point3<f32>, identifier: u32) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            identifier,
        }
    }

    pub fn buffer_layout() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array!(
                0 => Float32x3,
                1 => Uint32,
        );

        VertexBufferLayout {
            array_stride: size_of::<Self>() as _,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}
