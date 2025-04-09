use std::hash::{Hash, Hasher};

use bytemuck::{Pod, Zeroable};
use cgmath::{Point3, Vector2, Vector3};
use hashbrown::HashMap;
use wgpu::{VertexAttribute, VertexBufferLayout, VertexStepMode, vertex_attr_array};

use crate::Color;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Zeroable, Pod)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub texture_coordinates: [f32; 2],
    pub color: [f32; 3],
    pub texture_index: i32,
    pub wind_affinity: f32,
}

impl ModelVertex {
    const EPSILON: f32 = 1e-6;
}

impl PartialEq for ModelVertex {
    fn eq(&self, other: &Self) -> bool {
        (self.position[0] - other.position[0]).abs() < Self::EPSILON
            && (self.position[1] - other.position[1]).abs() < Self::EPSILON
            && (self.position[2] - other.position[2]).abs() < Self::EPSILON
            && (self.normal[0] - other.normal[0]).abs() < Self::EPSILON
            && (self.normal[1] - other.normal[1]).abs() < Self::EPSILON
            && (self.normal[2] - other.normal[2]).abs() < Self::EPSILON
            && (self.texture_coordinates[0] - other.texture_coordinates[0]).abs() < Self::EPSILON
            && (self.texture_coordinates[1] - other.texture_coordinates[1]).abs() < Self::EPSILON
            && (self.color[0] - other.color[0]).abs() < Self::EPSILON
            && (self.color[1] - other.color[1]).abs() < Self::EPSILON
            && (self.color[2] - other.color[2]).abs() < Self::EPSILON
            && self.texture_index == other.texture_index
            && (self.wind_affinity - other.wind_affinity).abs() < Self::EPSILON
    }
}

impl Eq for ModelVertex {}

impl Hash for ModelVertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let x = (self.position[0] / Self::EPSILON).round() as i32;
        let y = (self.position[1] / Self::EPSILON).round() as i32;
        let z = (self.position[2] / Self::EPSILON).round() as i32;
        x.hash(state);
        y.hash(state);
        z.hash(state);

        let nx = (self.normal[0] / Self::EPSILON).round() as i32;
        let ny = (self.normal[1] / Self::EPSILON).round() as i32;
        let nz = (self.normal[2] / Self::EPSILON).round() as i32;
        nx.hash(state);
        ny.hash(state);
        nz.hash(state);

        let tx = (self.texture_coordinates[0] / Self::EPSILON).round() as i32;
        let ty = (self.texture_coordinates[1] / Self::EPSILON).round() as i32;
        tx.hash(state);
        ty.hash(state);

        let r = (self.color[0] / Self::EPSILON).round() as i32;
        let g = (self.color[1] / Self::EPSILON).round() as i32;
        let b = (self.color[2] / Self::EPSILON).round() as i32;
        r.hash(state);
        g.hash(state);
        b.hash(state);

        let wind_affinity = (self.wind_affinity / Self::EPSILON).round() as i32;
        wind_affinity.hash(state);

        self.texture_index.hash(state);
    }
}

impl ModelVertex {
    pub const fn new(
        position: Point3<f32>,
        normal: Vector3<f32>,
        texture_coordinates: Vector2<f32>,
        color: Color,
        texture_index: i32,
        wind_affinity: f32,
    ) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            normal: [normal.x, normal.y, normal.z],
            texture_coordinates: [texture_coordinates.x, texture_coordinates.y],
            color: [color.red, color.green, color.blue],
            texture_index,
            wind_affinity,
        }
    }

    pub fn buffer_layout() -> VertexBufferLayout<'static> {
        static ATTRIBUTES: &[VertexAttribute] = &vertex_attr_array!(
                0 => Float32x3,
                1 => Float32x3,
                2 => Float32x2,
                3 => Float32x3,
                4 => Sint32,
                5 => Float32,
        );

        VertexBufferLayout {
            array_stride: size_of::<Self>() as _,
            step_mode: VertexStepMode::Vertex,
            attributes: ATTRIBUTES,
        }
    }
}

pub fn reduce_model_vertices(vertices: &[ModelVertex]) -> (Vec<ModelVertex>, Vec<u32>) {
    let mut vertex_map = HashMap::new();
    let mut reduced_vertices = Vec::new();
    let mut indices = Vec::new();

    for vertex in vertices.iter() {
        if let Some(&index) = vertex_map.get(vertex) {
            indices.push(index);
        } else {
            let index = reduced_vertices.len() as u32;
            vertex_map.insert(vertex, index);
            reduced_vertices.push(*vertex);
            indices.push(index);
        }
    }

    (reduced_vertices, indices)
}
