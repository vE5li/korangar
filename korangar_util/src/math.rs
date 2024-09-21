//! Implements some commonly used math functions.

use cgmath::{Matrix4, Vector3};

/// Multiplies a 4x4 matrix with a 3 component vector, treating the vector as a
/// point in 3D space.
pub fn multiply_matrix4_and_vector3(matrix: &Matrix4<f32>, vector: Vector3<f32>) -> Vector3<f32> {
    let adjusted_vector = matrix * vector.extend(1.0);
    (adjusted_vector / adjusted_vector.w).truncate()
}

#[cfg(test)]
mod tests {
    use cgmath::{assert_relative_eq, Matrix4, Vector3};

    use crate::math::multiply_matrix4_and_vector3;

    #[test]
    fn test_multiply_matrix4_and_vector3() {
        let translation = Vector3::new(1.0, 2.0, 3.0);
        let matrix = Matrix4::from_translation(translation);
        let vector = Vector3::new(4.0, 5.0, 6.0);
        let result = multiply_matrix4_and_vector3(&matrix, vector);
        assert_relative_eq!(result, Vector3::new(5.0, 7.0, 9.0), epsilon = 1e-6);
    }
}
