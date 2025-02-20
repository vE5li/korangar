use cgmath::{Array, InnerSpace, Matrix4, MetricSpace, Point3, Vector3};
#[cfg(feature = "interface")]
use korangar_interface::elements::PrototypeElement;

use crate::collision::{AABB, Insertable, Query};

/// A sphere.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "interface", derive(PrototypeElement))]
pub struct Sphere {
    center: Point3<f32>,
    radius: f32,
}

impl Sphere {
    /// Create a new sphere from a center point and radius.
    pub fn new(center: Point3<f32>, radius: f32) -> Self {
        Sphere { center, radius }
    }

    /// Creates the bounding sphere from an affine transformation matrix.
    pub fn from_transformation_matrix(transformation: Matrix4<f32>) -> Self {
        let center = Point3::from_homogeneous(transformation.w);
        let radius = transformation
            .x
            .magnitude()
            .max(transformation.y.magnitude())
            .max(transformation.z.magnitude());
        Sphere { center, radius }
    }

    /// Get the center of the sphere.
    pub fn center(&self) -> Point3<f32> {
        self.center
    }

    /// Get the radius of the sphere.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Get the diameter of the sphere.
    pub fn diameter(&self) -> f32 {
        self.radius * 2.0
    }

    /// Set the radius of the sphere.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius;
    }

    /// Set the diameter of the sphere.
    pub fn set_diameter(&mut self, diameter: f32) {
        self.radius = diameter * 0.5;
    }

    /// Check if a point is inside the sphere.
    pub fn contains_point(&self, point: Point3<f32>) -> bool {
        self.center.distance(point) <= self.radius
    }

    /// Check if this sphere intersects with another sphere.
    pub fn intersects_sphere(&self, other: &Sphere) -> bool {
        self.center.distance(other.center) <= self.radius + other.radius
    }

    /// Check if this sphere intersects with an AABB.
    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        let closest_point = Point3::new(
            self.center.x.clamp(aabb.min().x, aabb.max().x),
            self.center.y.clamp(aabb.min().y, aabb.max().y),
            self.center.z.clamp(aabb.min().z, aabb.max().z),
        );

        self.contains_point(closest_point)
    }

    /// Expand the sphere to include a point.
    pub fn expand(&mut self, point: Point3<f32>) {
        let distance = self.center.distance(point);
        if distance > self.radius {
            self.radius = distance;
        }
    }

    /// Merge this sphere with another sphere.
    pub fn merge(&self, other: &Sphere) -> Sphere {
        let center_diff = other.center - self.center;
        let distance = center_diff.magnitude();

        if distance + other.radius <= self.radius {
            *self
        } else if distance + self.radius <= other.radius {
            *other
        } else {
            let new_radius = (distance + self.radius + other.radius) * 0.5;
            let new_center = self.center + center_diff * ((new_radius - self.radius) / distance);
            Sphere::new(new_center, new_radius)
        }
    }
}

impl Insertable for Sphere {
    fn intersects_aabb(&self, aabb: &AABB) -> bool {
        self.intersects_aabb(aabb)
    }

    fn bounding_box(&self) -> AABB {
        AABB::from_center_and_size(self.center, Vector3::from_value(self.radius))
    }
}

impl Query<Sphere> for Sphere {
    fn intersects_aabb(&self, aabb: &AABB) -> bool {
        self.intersects_aabb(aabb)
    }

    fn intersects_object(&self, object: &Sphere) -> bool {
        self.intersects_sphere(object)
    }
}

impl Query<AABB> for Sphere {
    fn intersects_aabb(&self, aabb: &AABB) -> bool {
        self.intersects_aabb(aabb)
    }

    fn intersects_object(&self, object: &AABB) -> bool {
        self.intersects_aabb(object)
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{Matrix4, Point3};

    use crate::collision::{AABB, Query, Sphere};

    #[test]
    fn test_new() {
        let sphere = Sphere::new(Point3::new(1.0, 2.0, 3.0), 5.0);
        assert_eq!(sphere.center(), Point3::new(1.0, 2.0, 3.0));
        assert_eq!(sphere.radius(), 5.0);
    }

    #[test]
    fn test_from_transformation_matrix() {
        let translation = Matrix4::from_translation(cgmath::Vector3::new(1.0, 2.0, 3.0));
        let scale = Matrix4::from_scale(2.0);
        let transformation = translation * scale;
        let sphere = Sphere::from_transformation_matrix(transformation);
        assert_eq!(sphere.center(), Point3::new(1.0, 2.0, 3.0));
        assert_eq!(sphere.radius(), 2.0);
    }

    #[test]
    fn test_diameter() {
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 3.0);
        assert_eq!(sphere.diameter(), 6.0);
    }

    #[test]
    fn test_set_radius() {
        let mut sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0);
        sphere.set_radius(10.0);
        assert_eq!(sphere.radius(), 10.0);
    }

    #[test]
    fn test_set_diameter() {
        let mut sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0);
        sphere.set_diameter(20.0);
        assert_eq!(sphere.radius(), 10.0);
        assert_eq!(sphere.diameter(), 20.0);
    }

    #[test]
    fn test_contains_point() {
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0);
        assert!(sphere.contains_point(Point3::new(3.0, 4.0, 0.0)));
        assert!(!sphere.contains_point(Point3::new(4.0, 4.0, 0.0)));
    }

    #[test]
    fn test_intersects_sphere() {
        let sphere1 = Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0);
        let sphere2 = Sphere::new(Point3::new(7.0, 0.0, 0.0), 3.0);
        let sphere3 = Sphere::new(Point3::new(10.0, 0.0, 0.0), 3.0);
        assert!(sphere1.intersects_sphere(&sphere2));
        assert!(!sphere1.intersects_sphere(&sphere3));
    }

    #[test]
    fn test_intersects_aabb() {
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0);
        let aabb1 = AABB::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0));
        let aabb2 = AABB::new(Point3::new(5.0, 0.0, 0.0), Point3::new(6.0, 1.0, 1.0));
        let aabb3 = AABB::new(Point3::new(6.0, 6.0, 6.0), Point3::new(7.0, 7.0, 7.0));
        assert!(sphere.intersects_aabb(&aabb1));
        assert!(sphere.intersects_aabb(&aabb2));
        assert!(!sphere.intersects_aabb(&aabb3));
    }

    #[test]
    fn test_expand() {
        let mut sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0);
        sphere.expand(Point3::new(10.0, 0.0, 0.0));
        assert_eq!(sphere.radius(), 10.0);
    }

    #[test]
    fn test_merge() {
        let sphere1 = Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0);
        let sphere2 = Sphere::new(Point3::new(10.0, 0.0, 0.0), 3.0);
        let merged = sphere1.merge(&sphere2);
        assert_eq!(merged.center(), Point3::new(4.0, 0.0, 0.0));
        assert_eq!(merged.radius(), 9.0);
    }

    #[test]
    fn test_merge_contained_spheres() {
        let sphere1 = Sphere::new(Point3::new(0.0, 0.0, 0.0), 10.0);
        let sphere2 = Sphere::new(Point3::new(1.0, 1.0, 1.0), 5.0);
        let merged = sphere1.merge(&sphere2);
        assert_eq!(merged.center(), Point3::new(0.0, 0.0, 0.0));
        assert_eq!(merged.radius(), 10.0);

        let merged2 = sphere2.merge(&sphere1);
        assert_eq!(merged2.center(), Point3::new(0.0, 0.0, 0.0));
        assert_eq!(merged2.radius(), 10.0);
    }

    #[test]
    fn test_query_trait() {
        let sphere1 = Sphere::new(Point3::new(0.0, 0.0, 0.0), 5.0);
        let sphere2 = Sphere::new(Point3::new(7.0, 0.0, 0.0), 3.0);
        let aabb = AABB::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0));

        assert!(Sphere::intersects_object(&sphere1, &sphere2));
        assert!(Sphere::intersects_aabb(&sphere1, &aabb));
    }
}
