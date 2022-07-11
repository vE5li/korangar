pub use cgmath::{ Matrix4, Matrix3, Vector4, Vector3, Vector2, Point3, Rad, Deg, SquareMatrix, InnerSpace, Quaternion, Array };
pub use std::cmp::{ min, max };

pub fn multiply_matrix4_and_vector3(matrix: &Matrix4<f32>, vector: Vector3<f32>) -> Vector3<f32> {
    let adjusted_vector = matrix * vector.extend(1.0);
    adjusted_vector.truncate()
}
