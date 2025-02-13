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
    let threshold_angle = f32::cos(f32::to_radians(45.0));

    let mut normal_groups: HashMap<VertexPosition, Vec<Vector3<f32>>> = HashMap::new();

    // First pass: Collect normals into normal groups.
    for vertex in vertices.iter() {
        let position = VertexPosition::new(vertex.position);
        normal_groups.entry(position).or_default().push(vertex.normal);
    }

    // Second pass: Split groups by angle and average.
    let mut smoothed_normals: HashMap<VertexPosition, Vec<Vector3<f32>>> = HashMap::new();

    for (position, normals) in normal_groups.iter() {
        let processed = smoothed_normals.entry(*position).or_default();
        smooth_angle_based(normals, processed, threshold_angle);
    }

    // Final pass: Assign the closest smoothed normal.
    for vertex in vertices.iter_mut() {
        let position = VertexPosition::new(vertex.position);
        if let Some(normal_groups) = smoothed_normals.get(&position) {
            vertex.normal = find_best_normal(vertex.normal, normal_groups);
        }
    }
}

fn smooth_angle_based(normals: &[Vector3<f32>], processed: &mut Vec<Vector3<f32>>, threshold_angle: f32) {
    let mut normal_groups: Vec<Vec<Vector3<f32>>> = Vec::new();

    for &normal in normals.iter() {
        let mut added = false;

        for group in normal_groups.iter_mut() {
            if group.iter().all(|&n| normal.dot(n) > threshold_angle) {
                group.push(normal);
                added = true;
            }
        }

        if !added {
            normal_groups.push(vec![normal]);
        }
    }

    for group in normal_groups {
        let averaged = group.iter().fold(Vector3::zero(), |acc, n| acc + n).normalize();
        processed.push(averaged);
    }
}

fn find_best_normal(original: Vector3<f32>, candidates: &[Vector3<f32>]) -> Vector3<f32> {
    candidates
        .iter()
        .max_by(|&&a, &&b| {
            let dot_a = original.dot(a);
            let dot_b = original.dot(b);
            dot_a.total_cmp(&dot_b)
        })
        .copied()
        .unwrap_or(original)
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
