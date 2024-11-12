use cgmath::{Angle, Matrix4, Rad, Vector4};

/// The near-plane we use for all perspective projections.
pub const NEAR_PLANE: f32 = 1.0;

/// Calculates an orthographic projection matrix for WebGPU or DirectX
/// rendering.
///
/// This function generates a matrix that transforms from left-handed, y-up
/// world space to left-handed, y-up clip space with a depth range of 0.0 (near)
/// to 1.0 (far).
pub fn orthographic_lh(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Matrix4<f32> {
    let width = 1.0 / (right - left);
    let height = 1.0 / (top - bottom);
    let depth = 1.0 / (far - near);

    Matrix4::from_cols(
        Vector4::new(width + width, 0.0, 0.0, 0.0),
        Vector4::new(0.0, height + height, 0.0, 0.0),
        Vector4::new(0.0, 0.0, depth, 0.0),
        Vector4::new(-(left + right) * width, -(top + bottom) * height, -depth * near, 1.0),
    )
}

/// Calculates a perspective projection matrix for WebGPU or DirectX rendering.
///
/// This uses "reverse Z" with an infinite z-axis which helps greatly with Z
/// fighting and some approximate numerical computations.
///
/// This function generates a matrix that transforms from left-handed, y-up
/// world space to left-handed, y-up clip space with a depth range of 0.0 (near)
/// to 1.0 (far).
pub fn perspective_reverse_lh(vertical_fov: Rad<f32>, aspect_ratio: f32) -> Matrix4<f32> {
    let tangent = (vertical_fov / 2.0).tan();
    let height = 1.0 / tangent;
    let width = height / aspect_ratio;

    Matrix4::from_cols(
        Vector4::new(width, 0.0, 0.0, 0.0),
        Vector4::new(0.0, height, 0.0, 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
        Vector4::new(0.0, 0.0, NEAR_PLANE, 0.0),
    )
}
