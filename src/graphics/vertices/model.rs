use cgmath::{ Vector2, Vector3 };

#[derive(Default, Debug, Clone, Copy)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture_coordinates: [f32; 2],
    pub texture_index: i32,
}

impl ModelVertex {

    pub const fn new(position: Vector3<f32>, normal: Vector3<f32>, texture_coordinates: Vector2<f32>, texture_index: i32) -> Self {
        return Self {
            position: [position.x, position.y, position.z],
            normal: [normal.x, normal.y, normal.z],
            texture_coordinates: [texture_coordinates.x, texture_coordinates.y],
            texture_index: texture_index,
        }
    }
}

vulkano::impl_vertex!(ModelVertex, position, normal, texture_coordinates, texture_index);
