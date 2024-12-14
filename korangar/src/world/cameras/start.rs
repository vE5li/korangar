use cgmath::{Array, Deg, InnerSpace, Matrix4, Point3, Quaternion, Rad, Rotation, Rotation3, Vector2, Vector3, Zero};

use super::Camera;
use crate::graphics::perspective_reverse_lh;

const DEFAULT_VIEW_ANGLE: f32 = 180_f32.to_radians();
const DEFAULT_VIEW_DISTANCE: f32 = 150.0;
const ROTATION_SPEED: f32 = 0.03;
const VERTICAL_FOV: Deg<f32> = Deg(45.0);
const LOOK_UP: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

pub struct StartCamera {
    focus_point: Point3<f32>,
    camera_position: Point3<f32>,
    view_direction: Vector3<f32>,
    view_angle: f32,
    view_distance: f32,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
}

impl StartCamera {
    pub fn new() -> Self {
        Self {
            focus_point: Point3::from_value(0.0),
            camera_position: Point3::from_value(0.0),
            view_direction: Vector3::zero(),
            view_angle: DEFAULT_VIEW_ANGLE,
            view_distance: DEFAULT_VIEW_DISTANCE,
            view_matrix: Matrix4::zero(),
            projection_matrix: Matrix4::zero(),
            view_projection_matrix: Matrix4::zero(),
        }
    }

    pub fn set_focus_point(&mut self, focus_point: Point3<f32>) {
        self.focus_point = focus_point;
    }

    pub fn update(&mut self, delta_time: f64) {
        self.view_angle += delta_time as f32 * ROTATION_SPEED;

        let yaw_rotation = Quaternion::from_angle_y(Rad(self.view_angle));
        let base_offset = Vector3::new(0.0, self.view_distance, self.view_distance);
        let rotated_offset = yaw_rotation.rotate_vector(base_offset);

        self.camera_position = self.focus_point + rotated_offset;
        self.view_direction = -rotated_offset.normalize();
    }
}

impl Camera for StartCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        self.focus_point
    }

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        let aspect_ratio = window_size.x as f32 / window_size.y as f32;
        let camera_position = self.camera_position();
        self.view_matrix = Matrix4::look_to_lh(camera_position, self.view_direction, LOOK_UP);
        self.projection_matrix = perspective_reverse_lh(VERTICAL_FOV, aspect_ratio);
        self.view_projection_matrix = self.projection_matrix * self.view_matrix;
    }

    fn look_up_vector(&self) -> Vector3<f32> {
        LOOK_UP
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
