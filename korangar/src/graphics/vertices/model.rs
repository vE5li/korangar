use bytemuck::{Pod, Zeroable};
use cgmath::{Vector2, Vector3};
use vulkano::pipeline::graphics::vertex_input::Vertex;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod, Vertex)]
pub struct ModelVertex {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3],
    #[format(R32G32_SFLOAT)]
    pub texture_coordinates: [f32; 2],
    #[format(R32_SINT)]
    pub texture_index: i32,
    #[format(R32_SFLOAT)]
    pub wind_affinity: f32,
}

impl ModelVertex {
    pub const fn new(
        position: Vector3<f32>,
        normal: Vector3<f32>,
        texture_coordinates: Vector2<f32>,
        texture_index: i32,
        wind_affinity: f32,
    ) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            normal: [normal.x, normal.y, normal.z],
            texture_coordinates: [texture_coordinates.x, texture_coordinates.y],
            texture_index,
            wind_affinity,
        }
    }
}
