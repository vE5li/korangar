use cgmath::{Deg, Matrix4, Point3, Vector2, Vector3, Zero};

use super::Camera;
use crate::graphics::perspective_reverse_lh;

const VERTICAL_FOV: Deg<f32> = Deg(90.0);

pub struct PointShadowCamera {
    camera_position: Point3<f32>,
    view_direction: Vector3<f32>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
}

impl PointShadowCamera {
    pub fn new() -> Self {
        Self {
            camera_position: Point3::new(0.0, 0.0, 0.0),
            view_direction: Vector3::unit_x(),
            look_up_vector: Vector3::unit_y(),
            view_matrix: Matrix4::zero(),
            projection_matrix: Matrix4::zero(),
            view_projection_matrix: Matrix4::zero(),
        }
    }

    pub fn set_camera_position(&mut self, camera_position: Point3<f32>) {
        self.camera_position = camera_position;
    }

    pub fn change_direction(&mut self, direction: u32) {
        (self.view_direction, self.look_up_vector) = match direction {
            0 => (Vector3::unit_x(), Vector3::unit_y()),
            1 => (-Vector3::unit_x(), Vector3::unit_y()),
            2 => (Vector3::unit_y(), Vector3::unit_z()),
            3 => (-Vector3::unit_y(), -Vector3::unit_z()),
            4 => (Vector3::unit_z(), Vector3::unit_y()),
            5 => (-Vector3::unit_z(), Vector3::unit_y()),
            _ => panic!(),
        };
    }
}

impl Camera for PointShadowCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        self.camera_position + self.view_direction
    }

    fn generate_view_projection(&mut self, _window_size: Vector2<usize>) {
        self.view_matrix = Matrix4::look_to_lh(self.camera_position, self.view_direction, self.look_up_vector);
        self.projection_matrix = perspective_reverse_lh(VERTICAL_FOV, 1.0);
        self.view_projection_matrix = self.projection_matrix * self.view_matrix;
    }

    fn look_up_vector(&self) -> Vector3<f32> {
        self.look_up_vector
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)
    }

    fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.view_projection_matrix
    }

    fn view_direction(&self) -> Vector3<f32> {
        self.view_direction
    }
}
