//! Implements some commonly used math functions.

use cgmath::{EuclideanSpace, Matrix4, Point3};

/// Multiplies a 4x4 matrix with a 3 component vector, treating the vector as a
/// point in 3D space.
pub fn multiply_matrix4_and_point3(matrix: &Matrix4<f32>, vector: Point3<f32>) -> Point3<f32> {
    let adjusted_vector = matrix * vector.to_homogeneous();
    Point3::from_vec((adjusted_vector / adjusted_vector.w).truncate())
}

/// Simple linear interpolation.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use cgmath::{assert_relative_eq, EuclideanSpace, Matrix4, Point3};

    use crate::math::multiply_matrix4_and_point3;

    #[test]
    fn test_multiply_matrix4_and_point3() {
        let translation = Point3::new(1.0, 2.0, 3.0);
        let matrix = Matrix4::from_translation(translation.to_vec());
        let vector = Point3::new(4.0, 5.0, 6.0);
        let result = multiply_matrix4_and_point3(&matrix, vector);
        assert_relative_eq!(result, Point3::new(5.0, 7.0, 9.0), epsilon = 1e-6);
    }
}
