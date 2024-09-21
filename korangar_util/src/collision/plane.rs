use cgmath::{EuclideanSpace, InnerSpace, Point3, Vector3, Vector4};

use super::{Sphere, AABB};

/// Represents a plane in 3D space using the equation `Ax + By + Cz + D = 0`.
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    /// (A, B, C) components.
    normal: Vector3<f32>,
    /// D component.
    distance: f32,
}

impl Plane {
    /// Create a new plane from a normal vector and a point on the plane.
    pub fn new(normal: Vector3<f32>, point: Point3<f32>) -> Self {
        let normal = normal.normalize();
        Plane {
            normal,
            distance: -normal.dot(point.to_vec()),
        }
    }

    /// Creates a new plane from a Vector4 where (x, y, z) represent the normal
    /// and w represents the distance from the origin.
    pub fn from_vec4(v: Vector4<f32>) -> Self {
        Self {
            normal: v.truncate(),
            distance: v.w,
        }
    }

    /// Create a plane from three points.
    pub fn from_points(a: Point3<f32>, b: Point3<f32>, c: Point3<f32>) -> Self {
        let vector_1 = b - a;
        let vector_2 = c - a;
        let normal = vector_1.cross(vector_2).normalize();
        Self::new(normal, a)
    }

    /// Returns the normalized plane.
    pub fn normalized(&mut self) -> Self {
        let magnitude = self.normal.magnitude();
        Self {
            normal: self.normal / magnitude,
            distance: self.distance / magnitude,
        }
    }

    /// Get the normal of the plane.
    pub fn normal(&self) -> Vector3<f32> {
        self.normal
    }

    /// Get the distance of the plane from the origin.
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// Calculate the signed distance from a point to the plane.
    pub fn signed_distance_to_point(&self, point: Point3<f32>) -> f32 {
        self.normal.dot(point.to_vec()) + self.distance
    }

    /// Determine which side of the plane a point is on.
    pub fn classify_point(&self, point: Point3<f32>) -> IntersectionClassification {
        let distance = self.signed_distance_to_point(point);
        if distance > 0.0 {
            IntersectionClassification::Front
        } else if distance < 0.0 {
            IntersectionClassification::Back
        } else {
            IntersectionClassification::Intersecting
        }
    }

    /// Classify an AABB with respect to this plane.
    ///
    /// Based on "Real-Time Collision Detection" (2004), by Christer Ericson.
    /// Chapter 5.2.3: Testing Box Against Plane.
    pub fn classify_aabb(&self, aabb: &AABB) -> IntersectionClassification {
        let center = aabb.center();
        let extents = aabb.max() - center;

        // Compute the projection interval radius of b onto
        // L(t) = b.c + t * p.n
        let radius = extents.x * self.normal.x.abs() + extents.y * self.normal.y.abs() + extents.z * self.normal.z.abs();

        // Compute the distance of box center from plane.
        let distance = self.signed_distance_to_point(center);

        // Intersection occurs when the distance falls within [-r,+r] interval.
        if distance.abs() <= radius {
            IntersectionClassification::Intersecting
        } else if distance > 0.0 {
            IntersectionClassification::Front
        } else {
            IntersectionClassification::Back
        }
    }

    /// Classify a sphere with respect to this plane.
    pub fn classify_sphere(&self, sphere: &Sphere) -> IntersectionClassification {
        let center = sphere.center();
        let radius = sphere.radius();

        let distance = self.signed_distance_to_point(center);
        if distance > radius {
            IntersectionClassification::Front
        } else if distance < -radius {
            IntersectionClassification::Back
        } else {
            IntersectionClassification::Intersecting
        }
    }
}

/// Classifies the location against a plane.
#[derive(Debug, PartialEq, Eq)]
pub enum IntersectionClassification {
    /// Entirely in front of the plane.
    Front,
    /// Entirely behind the plane.
    Back,
    /// Intersects the plane.
    Intersecting,
}

#[cfg(test)]
mod tests {
    use cgmath::{assert_relative_eq, EuclideanSpace, InnerSpace, Point3, Vector3, Vector4};

    use crate::collision::{IntersectionClassification, Plane, Sphere, AABB};

    #[test]
    fn test_plane_new() {
        let normal = Vector3::unit_x();
        let point = Point3::new(5.0, 0.0, 0.0);
        let plane = Plane::new(normal, point);

        assert_relative_eq!(plane.normal(), Vector3::unit_x());
        assert_relative_eq!(plane.distance(), -5.0);
    }

    #[test]
    fn test_plane_from_points() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let plane = Plane::from_points(a, b, c);

        assert_relative_eq!(plane.normal(), Vector3::unit_z());
        assert_relative_eq!(plane.distance(), 0.0);
    }

    #[test]
    fn test_signed_distance_to_point() {
        let plane = Plane::new(Vector3::unit_y(), Point3::new(0.0, 5.0, 0.0));
        assert_relative_eq!(plane.signed_distance_to_point(Point3::new(0.0, 10.0, 0.0)), 5.0);
        assert_relative_eq!(plane.signed_distance_to_point(Point3::new(0.0, 0.0, 0.0)), -5.0);
        assert_relative_eq!(plane.signed_distance_to_point(Point3::new(0.0, 5.0, 0.0)), 0.0);
    }

    #[test]
    fn test_classify_point() {
        let plane = Plane::new(Vector3::unit_y(), Point3::new(0.0, 5.0, 0.0));
        assert_eq!(
            plane.classify_point(Point3::new(0.0, 10.0, 0.0)),
            IntersectionClassification::Front
        );
        assert_eq!(
            plane.classify_point(Point3::new(0.0, 0.0, 0.0)),
            IntersectionClassification::Back
        );
        assert_eq!(
            plane.classify_point(Point3::new(0.0, 5.0, 0.0)),
            IntersectionClassification::Intersecting
        );
    }

    #[test]
    fn test_classify_aabb() {
        let plane = Plane::new(Vector3::unit_y(), Point3::new(0.0, 5.0, 0.0));
        let aabb_front = AABB::new(Point3::new(0.0, 6.0, 0.0), Point3::new(1.0, 7.0, 1.0));
        let aabb_back = AABB::new(Point3::new(0.0, 3.0, 0.0), Point3::new(1.0, 4.0, 1.0));
        let aabb_intersecting = AABB::new(Point3::new(0.0, 4.0, 0.0), Point3::new(1.0, 6.0, 1.0));
        assert_eq!(plane.classify_aabb(&aabb_front), IntersectionClassification::Front);
        assert_eq!(plane.classify_aabb(&aabb_back), IntersectionClassification::Back);
        assert_eq!(
            plane.classify_aabb(&aabb_intersecting),
            IntersectionClassification::Intersecting
        );
    }

    #[test]
    fn test_classify_sphere() {
        let plane = Plane::new(Vector3::unit_y(), Point3::new(0.0, 5.0, 0.0));
        let sphere_front = Sphere::new(Point3::new(0.0, 10.0, 0.0), 2.0);
        let sphere_back = Sphere::new(Point3::new(0.0, 0.0, 0.0), 2.0);
        let sphere_intersecting = Sphere::new(Point3::new(0.0, 5.0, 0.0), 2.0);
        let sphere_touching_front = Sphere::new(Point3::new(0.0, 7.0, 0.0), 2.0);
        let sphere_touching_back = Sphere::new(Point3::new(0.0, 3.0, 0.0), 2.0);
        assert_eq!(plane.classify_sphere(&sphere_front), IntersectionClassification::Front);
        assert_eq!(plane.classify_sphere(&sphere_back), IntersectionClassification::Back);
        assert_eq!(
            plane.classify_sphere(&sphere_intersecting),
            IntersectionClassification::Intersecting
        );
        assert_eq!(
            plane.classify_sphere(&sphere_touching_front),
            IntersectionClassification::Intersecting
        );
        assert_eq!(
            plane.classify_sphere(&sphere_touching_back),
            IntersectionClassification::Intersecting
        );
    }

    #[test]
    fn test_plane_normal_normalization() {
        let normal = Vector3::new(3.0, 4.0, 5.0);
        let point = Point3::new(1.0, 2.0, 3.0);
        let plane = Plane::new(normal, point);
        assert_relative_eq!(plane.normal().magnitude(), 1.0);
    }

    #[test]
    fn test_plane_from_points_colinear() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 1.0, 1.0);
        let c = Point3::new(2.0, 2.0, 2.0);
        let plane = Plane::from_points(a, b, c);
        assert!(plane.normal().x.is_nan() || plane.normal().y.is_nan() || plane.normal().z.is_nan());
    }

    #[test]
    fn test_classify_point_edge_cases() {
        let plane = Plane::new(Vector3::unit_y(), Point3::origin());
        assert_eq!(
            plane.classify_point(Point3::new(0.0, 1e-10, 0.0)),
            IntersectionClassification::Front
        );
        assert_eq!(
            plane.classify_point(Point3::new(0.0, -1e-10, 0.0)),
            IntersectionClassification::Back
        );
    }

    #[test]
    fn test_classify_aabb_edge_cases() {
        let plane = Plane::new(Vector3::unit_y(), Point3::origin());
        let aabb_on_plane = AABB::new(Point3::new(-1.0, 0.0, -1.0), Point3::new(1.0, 0.0, 1.0));
        assert_eq!(plane.classify_aabb(&aabb_on_plane), IntersectionClassification::Intersecting);

        let aabb_touching = AABB::new(Point3::new(-1.0, 0.0, -1.0), Point3::new(1.0, 1e-6, 1.0));
        assert_eq!(plane.classify_aabb(&aabb_touching), IntersectionClassification::Intersecting);
    }

    #[test]
    fn test_plane_classify_aabb_degenerate() {
        let plane = Plane::new(Vector3::unit_y(), Point3::new(0.0, 5.0, 0.0));
        let aabb_degenerate = AABB::new(Point3::new(0.0, 5.0, 0.0), Point3::new(1.0, 5.0, 1.0));
        assert_eq!(plane.classify_aabb(&aabb_degenerate), IntersectionClassification::Intersecting);
    }

    #[test]
    fn test_plane_with_large_values() {
        let normal = Vector3::new(1e6, 2e6, 3e6);
        let point = Point3::new(1e9, 2e9, 3e9);
        let plane = Plane::new(normal, point);
        assert_relative_eq!(plane.normal().magnitude(), 1.0);

        let test_point = Point3::new(1e12, 2e12, 3e12);
        let distance = plane.signed_distance_to_point(test_point);
        assert!(!distance.is_nan() && !distance.is_infinite());
    }

    #[test]
    fn test_plane_from_vec4() {
        let vec4 = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let plane = Plane::from_vec4(vec4);
        assert_relative_eq!(plane.normal(), Vector3::new(1.0, 2.0, 3.0));
        assert_relative_eq!(plane.distance(), 4.0);
    }

    #[test]
    fn test_plane_normalized() {
        let mut plane = Plane::new(Vector3::new(3.0, 4.0, 0.0), Point3::new(0.0, 0.0, 0.0));
        let normalized_plane = plane.normalized();
        assert_relative_eq!(normalized_plane.normal().magnitude(), 1.0);
        assert_relative_eq!(normalized_plane.normal(), Vector3::new(0.6, 0.8, 0.0));
        assert_relative_eq!(normalized_plane.distance(), 0.0);
    }

    #[test]
    fn test_plane_normal_and_distance_getters() {
        let plane = Plane::new(Vector3::new(1.0, 2.0, 3.0), Point3::new(4.0, 5.0, 6.0));
        assert_relative_eq!(plane.normal(), Vector3::new(1.0, 2.0, 3.0).normalize());
        assert_relative_eq!(plane.distance(), -plane.normal().dot(Vector3::new(4.0, 5.0, 6.0)));
    }
}
