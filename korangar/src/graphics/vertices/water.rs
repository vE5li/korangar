use bytemuck::{Pod, Zeroable};
use cgmath::Vector3;
use wgpu::{vertex_attr_array, VertexAttribute, VertexBufferLayout, VertexStepMode};

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct WaterVertex {
    pub position: [f32; 3],
}

impl WaterVertex {
    pub fn new(position: Vector3<f32>) -> Self {
        Self { position: position.into() }
    }

    pub fn buffer_layout() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array!(
            0 => Float32x3,
        );

        VertexBufferLayout {
            array_stride: size_of::<Self>() as _,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}
