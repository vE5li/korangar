use bytemuck::{Pod, Zeroable};
use cgmath::{Point3, Vector2, Vector3};
use wgpu::{vertex_attr_array, VertexAttribute, VertexBufferLayout, VertexStepMode};

use crate::Color;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture_coordinates: [f32; 2],
    pub texture_index: i32,
    pub color: [f32; 3],
    pub wind_affinity: f32,
}

impl ModelVertex {
    pub const fn new(
        position: Point3<f32>,
        normal: Vector3<f32>,
        texture_coordinates: Vector2<f32>,
        texture_index: i32,
        color: Color,
        wind_affinity: f32,
    ) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            normal: [normal.x, normal.y, normal.z],
            texture_coordinates: [texture_coordinates.x, texture_coordinates.y],
            texture_index,
            color: [color.red, color.green, color.blue],
            wind_affinity,
        }
    }

    pub fn buffer_layout() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array!(
                0 => Float32x3,
                1 => Float32x3,
                2 => Float32x2,
                3 => Sint32,
                4 => Float32x3,
                5 => Float32,
        );

        VertexBufferLayout {
            array_stride: size_of::<Self>() as _,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}
