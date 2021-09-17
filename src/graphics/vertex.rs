use cgmath::{ Vector2, Vector3 };

#[derive(Default, Debug, Clone, Copy)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    texture_coordinates: [f32; 2],
}

impl Vertex {

    pub const fn new(position: Vector3<f32>, normal: Vector3<f32>, texture_coordinates: Vector2<f32>) -> Self {
        return Self {
            position: [position.x, position.y, position.z],
            normal: [normal.x, normal.y, normal.z],
            texture_coordinates: [texture_coordinates.x, texture_coordinates.y],
        }
    }
}

vulkano::impl_vertex!(Vertex, position, normal, texture_coordinates);
