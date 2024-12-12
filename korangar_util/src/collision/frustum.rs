use cgmath::{Matrix, Matrix4, Point3, Vector4};

use super::{IntersectionClassification, Plane, Sphere, AABB};
use crate::collision::Query;

/// The frustum used for frustum culling.
///
/// The normals of the planes are directed inside the frustum.
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    /// Constructs a new Frustum from a 4x4 transformation matrix
    /// using the Gribb-Hartmann method.
    ///
    /// This method efficiently extracts the 6 frustum planes from any 4x4
    /// matrix, including projection, view-projection, or
    /// model-view-projection matrices.
    ///
    /// The planes have the following order:
    ///
    /// - Left
    /// - Right
    /// - Bottom
    /// - Top
    /// - Near
    /// - Far
    pub fn new(matrix: Matrix4<f32>, reverse_z: bool) -> Self {
        let mut planes = [
            Plane::from_vec4(matrix.row(3) + matrix.row(0)).normalized(),
            Plane::from_vec4(matrix.row(3) - matrix.row(0)).normalized(),
            Plane::from_vec4(matrix.row(3) + matrix.row(1)).normalized(),
            Plane::from_vec4(matrix.row(3) - matrix.row(1)).normalized(),
            if reverse_z {
                Plane::from_vec4(matrix.row(3) - matrix.row(2)).normalized()
            } else {
                Plane::from_vec4(matrix.row(2)).normalized()
            },
            if reverse_z {
                Plane::from_vec4(matrix.row(2)).normalized()
            } else {
                Plane::from_vec4(matrix.row(3) - matrix.row(2)).normalized()
            },
        ];

        // Detect and handle infinite projection.
        if (!reverse_z && matrix.w.z == 0.0) || (reverse_z && matrix.z.z == 0.0) {
            let near_plane = planes[4];
            let far_plane = Plane::from_vec4(Vector4::new(
                -near_plane.normal().x,
                -near_plane.normal().y,
                -near_plane.normal().z,
                f32::INFINITY,
            ))
            .normalized();
            planes[5] = far_plane;
        }

        Self { planes }
    }

    /// Checks if a point is inside the frustum.
    pub fn contains_point(&self, point: Point3<f32>) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.classify_point(point) != IntersectionClassification::Back)
    }

    /// Test if the axis aligned bounding box is partially or fully inside the
    /// frustum.
    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.classify_aabb(aabb) != IntersectionClassification::Back)
    }

    /// Test if a sphere intersects with or is contained within the frustum.
    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        self.planes
            .iter()
            .all(|plane| plane.classify_sphere(sphere) != IntersectionClassification::Back)
    }
}

impl Query<AABB> for Frustum {
    fn intersects_aabb(&self, aabb: &AABB) -> bool {
        self.intersects_aabb(aabb)
    }

    fn intersects_object(&self, object: &AABB) -> bool {
        self.intersects_aabb(object)
    }
}

impl Query<Sphere> for Frustum {
    fn intersects_aabb(&self, aabb: &AABB) -> bool {
        self.intersects_aabb(aabb)
    }

    fn intersects_object(&self, sphere: &Sphere) -> bool {
        self.intersects_sphere(sphere)
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{Angle, Deg, Matrix4, Point3, Rad, Vector3, Vector4};

    use super::Frustum;
    use crate::collision::{Sphere, AABB};

    fn perspective_lh_zo(fovy: Rad<f32>, aspect: f32, near: f32, far: f32) -> Matrix4<f32> {
        let tan_half_fovy = (fovy / 2.0).tan();
        let f = 1.0 / tan_half_fovy;

        Matrix4::from_cols(
            Vector4::new(f / aspect, 0.0, 0.0, 0.0),
            Vector4::new(0.0, f, 0.0, 0.0),
            Vector4::new(0.0, 0.0, far / (far - near), 1.0),
            Vector4::new(0.0, 0.0, -near * far / (far - near), 0.0),
        )
    }

    fn perspective_reverse_lh(vertical_fov: Rad<f32>, aspect_ratio: f32, near: f32) -> Matrix4<f32> {
        let tangent = (vertical_fov / 2.0).tan();
        let height = 1.0 / tangent;
        let width = height / aspect_ratio;

        Matrix4::from_cols(
            Vector4::new(width, 0.0, 0.0, 0.0),
            Vector4::new(0.0, height, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector4::new(0.0, 0.0, near, 0.0),
        )
    }

    fn create_test_frustum() -> Frustum {
        let projection = perspective_lh_zo(Deg(90.0).into(), 1.0, 0.1, 100.0);
        let view = Matrix4::look_at_lh(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Vector3::new(0.0, 1.0, 0.0),
        );

        Frustum::new(projection * view, false)
    }

    #[test]
    fn test_points_outside_frustum() {
        let frustum = create_test_frustum();

        let test_points = [
            Point3::new(-2.0, 0.0, 1.0),  // Left
            Point3::new(2.0, 0.0, 1.0),   // Right
            Point3::new(0.0, -2.0, 1.0),  // Bottom
            Point3::new(0.0, 2.0, 1.0),   // Top
            Point3::new(0.0, 0.0, 0.05),  // Near
            Point3::new(0.0, 0.0, 150.0), // Far
        ];

        for (index, point) in test_points.iter().enumerate() {
            if frustum.contains_point(*point) {
                panic!(
                    "Point {} at {:?} should be outside the frustum plane {:?}",
                    index, point, frustum.planes[index]
                );
            }
        }
    }

    #[test]
    fn test_points_inside_frustum() {
        let frustum = create_test_frustum();

        let test_points = [
            Point3::new(-0.5, 0.0, 1.0), // Left
            Point3::new(0.5, 0.0, 1.0),  // Right
            Point3::new(0.0, -0.5, 1.0), // Bottom
            Point3::new(0.0, 0.5, 1.0),  // Top
            Point3::new(0.0, 0.0, 0.5),  // Near
            Point3::new(0.0, 0.0, 50.0), // Far
        ];

        for (index, point) in test_points.iter().enumerate() {
            if !frustum.contains_point(*point) {
                panic!(
                    "Point {} at {:?} should be inside the frustum plane {:?}",
                    index, point, frustum.planes[index]
                );
            }
        }
    }

    #[test]
    fn test_frustum_inside_large_aabb() {
        let frustum = create_test_frustum();
        let large_aabb = AABB::new(Point3::new(-1000.0, -1000.0, -1000.0), Point3::new(1000.0, 1000.0, 1000.0));
        assert!(frustum.intersects_aabb(&large_aabb), "Frustum should intersect with large AABB");
    }

    #[test]
    fn test_tiny_aabb_inside_frustum() {
        let frustum = create_test_frustum();
        let tiny_aabb = AABB::new(Point3::new(-0.1, -0.1, 1.0), Point3::new(0.1, 0.1, 1.1));
        assert!(frustum.intersects_aabb(&tiny_aabb), "Tiny AABB should be inside frustum");
    }

    #[test]
    fn test_aabb_intersects_far_plane() {
        let frustum = create_test_frustum();
        let intersecting_aabb = AABB::new(Point3::new(-1.0, -1.0, 99.0), Point3::new(1.0, 1.0, 101.0));
        assert!(frustum.intersects_aabb(&intersecting_aabb), "AABB should intersect far plane");
    }

    #[test]
    fn test_aabb_intersects_near_plane() {
        let frustum = create_test_frustum();
        let intersecting_aabb = AABB::new(Point3::new(-0.1, -0.1, 0.05), Point3::new(0.1, 0.1, 0.15));
        assert!(frustum.intersects_aabb(&intersecting_aabb), "AABB should intersect near plane");
    }

    #[test]
    fn test_aabb_outside_top_plane() {
        let frustum = create_test_frustum();
        let outside_aabb = AABB::new(Point3::new(-1.0, 51.0, 1.0), Point3::new(1.0, 52.0, -1.0));
        assert!(!frustum.intersects_aabb(&outside_aabb), "AABB should be outside far plane");
    }

    #[test]
    fn test_aabb_outside_bottom_plane() {
        let frustum = create_test_frustum();
        let outside_aabb = AABB::new(Point3::new(-1.0, -51.0, 1.0), Point3::new(1.0, -52.0, -1.0));
        assert!(!frustum.intersects_aabb(&outside_aabb), "AABB should be outside far plane");
    }

    #[test]
    fn test_aabb_outside_far_plane() {
        let frustum = create_test_frustum();
        let outside_aabb = AABB::new(Point3::new(-1.0, -1.0, 101.0), Point3::new(1.0, 1.0, 102.0));
        assert!(!frustum.intersects_aabb(&outside_aabb), "AABB should be outside far plane");
    }

    #[test]
    fn test_sphere_inside_frustum() {
        let frustum = create_test_frustum();
        assert!(frustum.intersects_sphere(&Sphere::new(Point3::new(0.0, 0.0, 1.0), 0.5)));
    }

    #[test]
    fn test_sphere_intersecting_frustum() {
        let frustum = create_test_frustum();
        assert!(frustum.intersects_sphere(&Sphere::new(Point3::new(0.0, 0.0, 0.5), 0.5)));
    }

    #[test]
    fn test_sphere_outside_frustum() {
        let frustum = create_test_frustum();
        assert!(!frustum.intersects_sphere(&Sphere::new(Point3::new(0.0, 0.0, -1.0), 0.5)));
    }

    #[test]
    fn test_large_sphere_containing_frustum() {
        let frustum = create_test_frustum();
        assert!(frustum.intersects_sphere(&Sphere::new(Point3::new(0.0, 0.0, 0.0), 1000.0)));
    }

    #[test]
    fn test_infinite_reverse_z_frustum() {
        let near = 10.0;
        let aspect_ratio = 16.0 / 9.0;
        let fov = Deg(60.0).into();

        let projection = perspective_reverse_lh(fov, aspect_ratio, near);
        let view = Matrix4::look_at_lh(
            Point3::new(0.0, 0.0, 50.0),
            Point3::new(0.0, 0.0, 51.0),
            Vector3::new(0.0, 1.0, 0.0),
        );

        let frustum = Frustum::new(projection * view, true);

        assert!(frustum.contains_point(Point3::new(0.0, 0.0, 61.0)));
        assert!(!frustum.contains_point(Point3::new(0.0, 0.0, 59.0)));

        assert!(frustum.contains_point(Point3::new(0.0, 0.0, 1000.0)));
        assert!(frustum.contains_point(Point3::new(0.0, 0.0, 10000.0)));
        assert!(frustum.contains_point(Point3::new(0.0, 0.0, 100000.0)));
    }
}
