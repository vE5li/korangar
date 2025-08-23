use cgmath::{Array, EuclideanSpace, Matrix4, Point3, Vector3};

use crate::collision::aligned_plane::{AlignedPlane, Axis};
use crate::collision::{Insertable, Query, Sphere};
use crate::math::multiply_matrix4_and_point3;

/// An axis aligned bounding box.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct AABB {
    pub(super) min: Point3<f32>,
    pub(super) max: Point3<f32>,
}

impl AABB {
    /// Create a new AABB from two points.
    pub fn new(point_0: Point3<f32>, point_1: Point3<f32>) -> Self {
        AABB {
            min: point_0.zip(point_1, f32::min),
            max: point_0.zip(point_1, f32::max),
        }
    }

    /// Calculates the axis aligned bounding box from a list of vertices.
    pub fn from_vertices<T>(vertex_positions: T) -> Self
    where
        T: IntoIterator<Item = Point3<f32>>,
    {
        let (min, max) = vertex_positions.into_iter().fold(
            (Point3::from_value(f32::MAX), Point3::from_value(f32::MIN)),
            |(min, max), position| (min.zip(position, f32::min), max.zip(position, f32::max)),
        );

        Self { min, max }
    }

    /// Create an AABB from a center point and half-extents.
    pub fn from_center_and_size(center: Point3<f32>, half_size: Vector3<f32>) -> Self {
        AABB {
            min: center - half_size,
            max: center + half_size,
        }
    }

    /// Creates the bounding box from an affine transformation matrix.
    pub fn from_transformation_matrix(transformation: Matrix4<f32>) -> AABB {
        // Define 4 corners of the unit cube that cover
        // all combinations of min/max per axis.
        let corners = [
            Point3::new(-1.0, -1.0, -1.0),
            Point3::new(-1.0, 1.0, 1.0),
            Point3::new(1.0, -1.0, 1.0),
            Point3::new(1.0, 1.0, -1.0),
        ];

        let transformed_corners = corners.map(|corner| multiply_matrix4_and_point3(&transformation, corner));

        Self::from_vertices(transformed_corners)
    }

    /// Creates a point without a meaningful value.
    pub fn uninitialized() -> Self {
        let min: Point3<f32> = Point3::from_value(f32::MAX);
        let max: Point3<f32> = Point3::from_value(-f32::MAX);

        Self { min, max }
    }

    /// Get the min point of the AABB.
    pub fn min(&self) -> Point3<f32> {
        self.min
    }

    /// Get the max point of the AABB.
    pub fn max(&self) -> Point3<f32> {
        self.max
    }

    /// Get the center of the AABB.
    pub fn center(&self) -> Point3<f32> {
        (self.min + self.max.to_vec()) * 0.5
    }

    /// Get the size (dimensions) of the AABB.
    pub fn size(&self) -> Vector3<f32> {
        self.max - self.min
    }

    /// Check if a point is inside the AABB.
    pub fn contains_point(&self, point: Point3<f32>) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Check if this AABB intersects with a sphere.
    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        sphere.intersects_aabb(self)
    }

    /// Check if this AABB intersects with another AABB.
    pub fn intersects_aabb(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Creates a new AABB that is expanded by a given margin in all directions.
    pub fn expanded(&self, margin: f32) -> Self {
        AABB {
            min: Point3::new(self.min.x - margin, self.min.y - margin, self.min.z - margin),
            max: Point3::new(self.max.x + margin, self.max.y + margin, self.max.z + margin),
        }
    }

    /// Expand the AABB to include a point.
    pub fn expand(&mut self, point: Point3<f32>) {
        self.min = self.min.zip(point, f32::min);
        self.max = self.max.zip(point, f32::max);
    }

    /// Merge this AABB with another AABB.
    pub fn merge(&self, other: &AABB) -> AABB {
        AABB {
            min: self.min.zip(other.min, f32::min),
            max: self.max.zip(other.max, f32::max),
        }
    }

    /// Extends the current AABB with another AABB.
    pub fn extend(&mut self, other: &Self) {
        self.min = self.min.zip(other.min, f32::min);
        self.max = self.max.zip(other.max, f32::max);
    }

    /// Calculates the surface of an AABB.
    pub fn surface(&self) -> f32 {
        let width = self.max.x - self.min.x;
        let height = self.max.y - self.min.y;
        let depth = self.max.z - self.min.z;
        (width * height + width * depth + height * depth) * 2.0
    }

    /// Splits the AABB along the splitting plane and returns the resulting two
    /// AABB.
    pub fn split(&self, splitting_plane: &AlignedPlane) -> (AABB, AABB) {
        let mut left = *self;
        let mut right = *self;
        let distance = splitting_plane.distance();
        match splitting_plane.axis() {
            Axis::X => {
                right.min.x = distance.clamp(self.min().x, self.max().x);
                left.max.x = distance.clamp(self.min().x, self.max().x);
            }
            Axis::Y => {
                right.min.y = distance.clamp(self.min.y, self.max.y);
                left.max.y = distance.clamp(self.min.y, self.max.y);
            }
            Axis::Z => {
                right.min.z = distance.clamp(self.min.z, self.max.z);
                left.max.z = distance.clamp(self.min.z, self.max.z);
            }
        }
        (left, right)
    }
}

impl Insertable for AABB {
    fn intersects_aabb(&self, aabb: &AABB) -> bool {
        self.intersects_aabb(aabb)
    }

    fn bounding_box(&self) -> AABB {
        *self
    }
}

impl Query<AABB> for AABB {
    fn intersects_aabb(&self, aabb: &AABB) -> bool {
        self.intersects_aabb(aabb)
    }

    fn intersects_object(&self, object: &AABB) -> bool {
        self.intersects_aabb(object)
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{Matrix4, Point3, Vector3};

    use crate::collision::{AABB, AlignedPlane, Axis, Sphere};

    #[test]
    fn test_new() {
        let aabb = AABB::new(Point3::new(1.0, 2.0, 3.0), Point3::new(4.0, 5.0, 6.0));
        let aabb_reversed = AABB::new(Point3::new(4.0, 5.0, 6.0), Point3::new(1.0, 2.0, 3.0));

        assert_eq!(aabb.min(), Point3::new(1.0, 2.0, 3.0));
        assert_eq!(aabb.max(), Point3::new(4.0, 5.0, 6.0));

        assert_eq!(aabb_reversed.min(), Point3::new(1.0, 2.0, 3.0));
        assert_eq!(aabb_reversed.max(), Point3::new(4.0, 5.0, 6.0));
    }

    #[test]
    fn test_from_center_and_size() {
        let center = Point3::new(0.0, 0.0, 0.0);
        let half_size = Vector3::new(1.0, 1.0, 1.0);
        let aabb = AABB::from_center_and_size(center, half_size);

        assert_eq!(aabb.min(), Point3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max(), Point3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_center() {
        let aabb = AABB::new(Point3::new(-1.0, -2.0, -3.0), Point3::new(1.0, 2.0, 3.0));

        assert_eq!(aabb.center(), Point3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_size() {
        let aabb = AABB::new(Point3::new(-1.0, -2.0, -3.0), Point3::new(1.0, 2.0, 3.0));

        assert_eq!(aabb.size(), Vector3::new(2.0, 4.0, 6.0));
    }

    #[test]
    fn test_contains_point() {
        let aabb = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));

        assert!(aabb.contains_point(Point3::new(1.0, 1.0, 1.0)));
        assert!(aabb.contains_point(Point3::new(0.0, 0.0, 0.0)));
        assert!(aabb.contains_point(Point3::new(2.0, 2.0, 2.0)));
        assert!(!aabb.contains_point(Point3::new(-1.0, 1.0, 1.0)));
        assert!(!aabb.contains_point(Point3::new(3.0, 1.0, 1.0)));
    }

    #[test]
    fn test_intersects() {
        let aabb_1 = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0));
        let aabb_2 = AABB::new(Point3::new(1.0, 1.0, 1.0), Point3::new(3.0, 3.0, 3.0));
        let aabb_3 = AABB::new(Point3::new(3.0, 3.0, 3.0), Point3::new(4.0, 4.0, 4.0));

        assert!(aabb_1.intersects_aabb(&aabb_2));
        assert!(aabb_2.intersects_aabb(&aabb_1));
        assert!(!aabb_1.intersects_aabb(&aabb_3));
        assert!(aabb_2.intersects_aabb(&aabb_3));
    }

    #[test]
    fn test_expand() {
        let mut aabb = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        aabb.expand(Point3::new(2.0, -1.0, 0.5));

        assert_eq!(aabb.min(), Point3::new(0.0, -1.0, 0.0));
        assert_eq!(aabb.max(), Point3::new(2.0, 1.0, 1.0));
    }

    #[test]
    fn test_merge() {
        let aabb_1 = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let aabb_2 = AABB::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(2.0, 2.0, 2.0));
        let merged = aabb_1.merge(&aabb_2);

        assert_eq!(merged.min(), Point3::new(-1.0, -1.0, -1.0));
        assert_eq!(merged.max(), Point3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_aabb_with_negative_dimensions() {
        let aabb = AABB::new(Point3::new(2.0, 2.0, 2.0), Point3::new(1.0, 1.0, 1.0));

        assert_eq!(aabb.min(), Point3::new(1.0, 1.0, 1.0));
        assert_eq!(aabb.max(), Point3::new(2.0, 2.0, 2.0));
        assert_eq!(aabb.size(), Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_aabb_intersects_edge_cases() {
        let aabb_1 = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let aabb_2 = AABB::new(Point3::new(1.0, 1.0, 1.0), Point3::new(2.0, 2.0, 2.0));
        let aabb_3 = AABB::new(Point3::new(1.0, 1.0, 1.0), Point3::new(1.0, 1.0, 1.0));

        assert!(aabb_1.intersects_aabb(&aabb_2));
        assert!(aabb_1.intersects_aabb(&aabb_3));
        assert!(aabb_2.intersects_aabb(&aabb_3));
    }

    #[test]
    fn test_from_vertices() {
        let vertices = vec![Point3::new(1.0, 2.0, 3.0), Point3::new(-1.0, 4.0, 0.0), Point3::new(2.0, -2.0, 5.0)];
        let aabb = AABB::from_vertices(vertices);

        assert_eq!(aabb.min(), Point3::new(-1.0, -2.0, 0.0));
        assert_eq!(aabb.max(), Point3::new(2.0, 4.0, 5.0));
    }

    #[test]
    fn test_from_transformation_matrix() {
        let translation = Vector3::new(1.0, 2.0, 3.0);
        let scale = Vector3::new(2.0, 2.0, 2.0);
        let transformation = Matrix4::from_translation(translation) * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
        let aabb = AABB::from_transformation_matrix(transformation);

        assert_eq!(aabb.min(), Point3::new(-1.0, 0.0, 1.0));
        assert_eq!(aabb.max(), Point3::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn test_uninitialized() {
        let aabb = AABB::uninitialized();

        assert_eq!(aabb.min(), Point3::new(f32::MAX, f32::MAX, f32::MAX));
        assert_eq!(aabb.max(), Point3::new(-f32::MAX, -f32::MAX, -f32::MAX));
    }

    #[test]
    fn test_expanded() {
        let aabb = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let expanded = aabb.expanded(0.5);

        assert_eq!(expanded.min(), Point3::new(-0.5, -0.5, -0.5));
        assert_eq!(expanded.max(), Point3::new(1.5, 1.5, 1.5));
    }

    #[test]
    fn test_extend() {
        let mut aabb_1 = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0));
        let aabb_2 = AABB::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(2.0, 2.0, 2.0));
        aabb_1.extend(&aabb_2);

        assert_eq!(aabb_1.min(), Point3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb_1.max(), Point3::new(2.0, 2.0, 2.0));
    }

    #[test]
    fn test_intersects_sphere() {
        let aabb = AABB::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0));
        let sphere_inside = Sphere::new(Point3::new(0.0, 0.0, 0.0), 0.5);
        let sphere_intersecting = Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.5);
        let sphere_outside = Sphere::new(Point3::new(3.0, 3.0, 3.0), 0.5);

        assert!(aabb.intersects_sphere(&sphere_inside));
        assert!(aabb.intersects_sphere(&sphere_intersecting));
        assert!(!aabb.intersects_sphere(&sphere_outside));
    }

    #[test]
    fn test_split() {
        let aabb = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 4.0, 4.0));
        let plane = AlignedPlane::new(Axis::X, 2.0);
        let (left, right) = aabb.split(&plane);

        assert_eq!(left.min(), Point3::new(0.0, 0.0, 0.0));
        assert_eq!(left.max(), Point3::new(2.0, 4.0, 4.0));
        assert_eq!(right.min(), Point3::new(2.0, 0.0, 0.0));
        assert_eq!(right.max(), Point3::new(4.0, 4.0, 4.0));
    }

    #[test]
    fn test_split_outside_max() {
        let aabb = AABB::new(Point3::new(1.0, 1.0, 1.0), Point3::new(4.0, 4.0, 4.0));
        let plane = AlignedPlane::new(Axis::Y, 5.0);
        let (left, right) = aabb.split(&plane);

        assert_eq!(left.min(), Point3::new(1.0, 1.0, 1.0));
        assert_eq!(left.max(), Point3::new(4.0, 4.0, 4.0));
        assert_eq!(right.min(), Point3::new(1.0, 4.0, 1.0));
        assert_eq!(right.max(), Point3::new(4.0, 4.0, 4.0));
    }
}
