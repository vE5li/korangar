use crate::collision::AABB;

/// An axis in 3D space
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum Axis {
    /// X Axis.
    X = 0,
    /// Y Axis.
    Y = 1,
    /// Z Axis.
    Z = 2,
}

/// An axis aligned plane.
#[derive(Debug, Clone, Copy)]
pub struct AlignedPlane {
    axis: Axis,
    distance: f32,
}

impl AlignedPlane {
    /// Creates a new axis aligned plane from the given axis and distance.
    pub const fn new(axis: Axis, distance: f32) -> Self {
        AlignedPlane { axis, distance }
    }

    /// Tests if the plane intersects an axis aligned bounding box.
    pub fn intersects_aabb(&self, space: &AABB) -> bool {
        match self.axis {
            Axis::X => self.distance > space.min().x && self.distance < space.max().x,
            Axis::Y => self.distance > space.min().y && self.distance < space.max().y,
            Axis::Z => self.distance > space.min().z && self.distance < space.max().z,
        }
    }

    /// Returns the axis of the plane.
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// Returns the distance of the plane.
    pub fn distance(&self) -> f32 {
        self.distance
    }
}

#[cfg(test)]
mod tests {
    use cgmath::Point3;

    use crate::collision::{AlignedPlane, Axis, AABB};

    #[test]
    fn test_aligned_plane_creation() {
        let plane_x = AlignedPlane::new(Axis::X, 2.0);
        assert_eq!(plane_x.axis(), Axis::X);
        assert_eq!(plane_x.distance(), 2.0);

        let plane_y = AlignedPlane::new(Axis::Y, -1.5);
        assert_eq!(plane_y.axis(), Axis::Y);
        assert_eq!(plane_y.distance(), -1.5);

        let plane_z = AlignedPlane::new(Axis::Z, 0.0);
        assert_eq!(plane_z.axis(), Axis::Z);
        assert_eq!(plane_z.distance(), 0.0);
    }

    #[test]
    fn test_aligned_plane_intersects_aabb() {
        let aabb = AABB::new(Point3::new(0.0, 0.0, 0.0), Point3::new(4.0, 4.0, 4.0));

        let plane_intersecting_x = AlignedPlane::new(Axis::X, 2.0);
        assert!(plane_intersecting_x.intersects_aabb(&aabb));

        let plane_intersecting_y = AlignedPlane::new(Axis::Y, 3.0);
        assert!(plane_intersecting_y.intersects_aabb(&aabb));

        let plane_intersecting_z = AlignedPlane::new(Axis::Z, 1.0);
        assert!(plane_intersecting_z.intersects_aabb(&aabb));

        let plane_not_intersecting_x = AlignedPlane::new(Axis::X, 5.0);
        assert!(!plane_not_intersecting_x.intersects_aabb(&aabb));

        let plane_not_intersecting_y = AlignedPlane::new(Axis::Y, -1.0);
        assert!(!plane_not_intersecting_y.intersects_aabb(&aabb));

        let plane_not_intersecting_z = AlignedPlane::new(Axis::Z, 4.5);
        assert!(!plane_not_intersecting_z.intersects_aabb(&aabb));
    }
}
