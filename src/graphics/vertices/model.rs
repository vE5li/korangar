use cgmath::{Vector2, Vector3};

#[derive(Default, Debug, Clone, Copy)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture_coordinates: [f32; 2],
    pub texture_index: i32,
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

vulkano::impl_vertex!(ModelVertex, position, normal, texture_coordinates, texture_index, wind_affinity);
