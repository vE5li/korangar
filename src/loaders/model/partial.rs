use cgmath::{ Vector2, Vector3 };

use graphics::Vertex;

pub struct PartialVertex {
    pub position: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub texture_coordinates: Vector2<f32>,
    pub texture_index: i32,
}

impl PartialVertex {

    pub fn new(position: Vector3<f32>, normal: Vector3<f32>, texture_coordinates: Vector2<f32>, texture_index: i32) -> Self {
        return Self { position, normal, texture_coordinates, texture_index };
    }

    pub fn to_vertex(self) -> Vertex {
        return Vertex::new(self.position, self.normal, self.texture_coordinates, self.texture_index);
    }
}

pub fn calculate_normal(first_position: Vector3<f32>, second_position: Vector3<f32>, third_position: Vector3<f32>) -> Vector3<f32> {
    let delta_position_1 = second_position - first_position;
    let delta_position_2 = third_position - first_position;
    return delta_position_1.cross(delta_position_2);
}
