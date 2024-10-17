use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use derive_new::new;

use crate::graphics::{Color, ModelVertex};

#[derive(new)]
pub struct NativeModelVertex {
    pub position: Point3<f32>,
    pub normal: Vector3<f32>,
    pub texture_coordinates: Vector2<f32>,
    pub texture_index: i32,
    pub color: Color,
    pub wind_affinity: f32,
}

impl NativeModelVertex {
    fn convert_to_vertex(self, texture_index_mapping: Option<&[i32]>) -> ModelVertex {
        // sic! We want to panic when the mapping doesn't contain the texture index.
        let texture_index = texture_index_mapping
            .map(|mapping| mapping[self.texture_index as usize])
            .unwrap_or(self.texture_index);
        ModelVertex::new(
            self.position,
            self.normal,
            self.texture_coordinates,
            texture_index,
            self.color,
            self.wind_affinity,
        )
    }

    pub fn to_vertices(mut native_vertices: Vec<NativeModelVertex>, texture_index_mapping: Option<&[i32]>) -> Vec<ModelVertex> {
        let mut vertices = Vec::new();
        let mut drain_iterator = native_vertices.drain(..);

        while let Some(mut first_partial) = drain_iterator.next() {
            let mut second_partial = drain_iterator.next().unwrap();
            let mut third_partial = drain_iterator.next().unwrap();

            first_partial.normal = first_partial.normal.normalize();
            second_partial.normal = second_partial.normal.normalize();
            third_partial.normal = third_partial.normal.normalize();

            vertices.push(first_partial.convert_to_vertex(texture_index_mapping));
            vertices.push(second_partial.convert_to_vertex(texture_index_mapping));
            vertices.push(third_partial.convert_to_vertex(texture_index_mapping));
        }

        vertices
    }

    pub fn calculate_normal(first_position: Point3<f32>, second_position: Point3<f32>, third_position: Point3<f32>) -> Vector3<f32> {
        let delta_position_1 = second_position - first_position;
        let delta_position_2 = third_position - first_position;
        delta_position_1.cross(delta_position_2)
    }
}
