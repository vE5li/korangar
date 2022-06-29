use derive_new::new;
use cgmath::{ Vector2, Vector3, InnerSpace };

use graphics::ModelVertex;

#[derive(new)]
pub struct NativeModelVertex {
    pub position: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub texture_coordinates: Vector2<f32>,
    pub texture_index: i32,
}

impl NativeModelVertex {

    fn to_vertex(self) -> ModelVertex {
        ModelVertex::new(self.position, self.normal, self.texture_coordinates, self.texture_index)
    }

    pub fn to_vertices(mut native_vertices: Vec<NativeModelVertex>) -> Vec<ModelVertex> {

        let mut vertices = Vec::new();
        let mut drain_iterator = native_vertices.drain(..);

        while let Some(mut first_partial) = drain_iterator.next() {
            let mut second_partial = drain_iterator.next().unwrap();
            let mut third_partial = drain_iterator.next().unwrap();

            first_partial.normal = first_partial.normal.normalize();
            second_partial.normal = second_partial.normal.normalize();
            third_partial.normal = third_partial.normal.normalize();

            vertices.push(first_partial.to_vertex());
            vertices.push(second_partial.to_vertex());
            vertices.push(third_partial.to_vertex());
        }

        vertices
    }

    pub fn calculate_normal(first_position: Vector3<f32>, second_position: Vector3<f32>, third_position: Vector3<f32>) -> Vector3<f32> {
        let delta_position_1 = second_position - first_position;
        let delta_position_2 = third_position - first_position;
        delta_position_1.cross(delta_position_2)
    }
}
