use std::hash::{Hash, Hasher};

use cgmath::{InnerSpace, Point3, Vector3, Zero};
use hashbrown::HashMap;

use crate::graphics::NativeModelVertex;

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
struct VertexPosition(Point3<f32>);

impl VertexPosition {
    const EPSILON: f32 = 1e-6;

    fn new(point: Point3<f32>) -> Self {
        Self(point)
    }
}

impl PartialEq for VertexPosition {
    fn eq(&self, other: &Self) -> bool {
        (self.0.x - other.0.x).abs() < Self::EPSILON
            && (self.0.y - other.0.y).abs() < Self::EPSILON
            && (self.0.z - other.0.z).abs() < Self::EPSILON
    }
}

impl Eq for VertexPosition {}

impl Hash for VertexPosition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let x = (self.0.x / Self::EPSILON).round() as i32;
        let y = (self.0.y / Self::EPSILON).round() as i32;
        let z = (self.0.z / Self::EPSILON).round() as i32;
        x.hash(state);
        y.hash(state);
        z.hash(state);
    }
}

pub fn smooth_ground_normals(vertices: &mut [NativeModelVertex]) {
    const EPSILON: f32 = 1e-6;

    // Artificial vertices connect to an edge, which is parallel to the Y axis.
    // Such edges should only occur in artificial structures like walls.
    let mut artificial_vertices = vec![false; vertices.len()];

    for (chunk_index, chunk) in vertices.chunks_mut(3).enumerate().filter(|(_, chunk)| chunk.len() == 3) {
        for vertex_index in 0..3 {
            let index0 = vertex_index;
            let index1 = (vertex_index + 1) % 3;

            let position0 = chunk[index0].position;
            let position1 = chunk[index1].position;

            if (position0.x - position1.x).abs() < EPSILON && (position0.z - position1.z).abs() < EPSILON {
                artificial_vertices[chunk_index * 3 + index0] = true;
                artificial_vertices[chunk_index * 3 + index1] = true;
            }
        }
    }

    let mut normals: HashMap<VertexPosition, Vector3<f32>> = HashMap::new();

    for (vertex, _) in vertices.iter().zip(artificial_vertices).filter(|(_, is_artificial)| !is_artificial) {
        let position = VertexPosition::new(vertex.position);
        *normals.entry(position).or_insert_with(Vector3::zero) += vertex.normal;
    }

    for (_, normals) in normals.iter_mut() {
        *normals = normals.normalize();
    }

    for vertex in vertices.iter_mut() {
        let position = VertexPosition::new(vertex.position);
        if let Some(&smooth_normal) = normals.get(&position) {
            vertex.normal = smooth_normal;
        }
    }
}

pub fn smooth_model_normals(vertices: &mut [NativeModelVertex]) {
    let mut smooth_group_normals: Vec<HashMap<VertexPosition, Vec<Vector3<f32>>>> = Vec::default();

    // First pass: Collect normals into normal groups for each smoothing group.
    for vertex in vertices.iter() {
        let position = VertexPosition::new(vertex.position);

        for &group in vertex.smoothing_groups.iter() {
            if group.is_negative() {
                // Smooth groups are sorted, so we can exit early.
                break;
            }

            match smooth_group_normals.get_mut(group as usize) {
                Some(group_normals) => {
                    group_normals.entry(position).or_default().push(vertex.normal);
                }
                None => {
                    let group = group % 128; // Arbitrary limit for safety.
                    smooth_group_normals.resize((group + 1) as usize, HashMap::default());
                    smooth_group_normals[group as usize]
                        .entry(position)
                        .or_default()
                        .push(vertex.normal);
                }
            }
        }
    }

    // Second pass: Sum all normal groups of each smoothing group.
    let smooth_group_normals: Vec<HashMap<VertexPosition, Vector3<f32>>> = smooth_group_normals
        .iter_mut()
        .map(|group_normals| {
            let mut smoothed_group_normals = HashMap::with_capacity(group_normals.len());

            for (position, group) in group_normals.iter() {
                let sum = group.iter().fold(Vector3::zero(), |acc, n| acc + n);
                smoothed_group_normals.insert(*position, sum);
            }

            smoothed_group_normals
        })
        .collect();

    // Final pass: Average the smooth group normals of a vertex and assign it
    for vertex in vertices.iter_mut() {
        let position = VertexPosition::new(vertex.position);

        let mut sum = Vector3::<f32>::zero();

        for &group in vertex.smoothing_groups.iter() {
            if group.is_negative() {
                // Smooth groups are sorted, so we can exit early.
                break;
            }

            smooth_group_normals[group as usize].get(&position).iter().for_each(|&&normal| {
                sum += normal;
            });
        }

        vertex.normal = sum.normalize();
    }
}
