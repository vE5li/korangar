use std::hash::{Hash, Hasher};

use cgmath::{InnerSpace, Point3, Vector3};
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
    // Threshold to prevent smoothing of hard edges (cos(45°))
    const ANGLE_THRESHOLD_COS: f32 = std::f32::consts::FRAC_1_SQRT_2;

    let mut normal_groups: HashMap<VertexPosition, Vec<Vector3<f32>>> = HashMap::new();

    // First pass: Collect normals into groups
    for vertex in vertices.iter() {
        let position = VertexPosition::new(vertex.position);
        normal_groups.entry(position).or_default().push(vertex.normal);
    }

    // Second pass: Split groups by angle and average
    let mut smoothed_normals: HashMap<VertexPosition, Vec<Vector3<f32>>> = HashMap::new();

    for (position, normals) in normal_groups {
        let processed = smoothed_normals.entry(position).or_default();

        'normal_loop: for &normal in normals.iter() {
            for group in processed.iter_mut() {
                if normal.dot(*group) > ANGLE_THRESHOLD_COS {
                    *group = (*group + normal).normalize();
                    continue 'normal_loop;
                }
            }

            processed.push(normal);
        }
    }

    // Final pass: Assign the closest smoothed normal
    for vertex in vertices.iter_mut() {
        let position = VertexPosition::new(vertex.position);
        if let Some(normal_groups) = smoothed_normals.get(&position) {
            let closest_normal = normal_groups
                .iter()
                .max_by(|&&a, &&b| {
                    let dot_a = vertex.normal.dot(a);
                    let dot_b = vertex.normal.dot(b);
                    dot_a.total_cmp(&dot_b)
                })
                .copied()
                .unwrap_or(vertex.normal);

            vertex.normal = closest_normal;
        }
    }
}
