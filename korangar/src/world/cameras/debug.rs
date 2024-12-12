use cgmath::{Deg, InnerSpace, Matrix4, Point3, Quaternion, Rad, Rotation, Rotation3, Vector2, Vector3, Zero};

use super::Camera;
use crate::graphics::perspective_reverse_lh;

const LOOK_AROUND_SPEED: f32 = 0.005;
const FLY_SPEED_FAST: f32 = 1000.0;
const FLY_SPEED_SLOW: f32 = 100.0;
const VERTICAL_FOV: Deg<f32> = Deg(45.0);
const LOOK_UP_VECTOR: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

pub struct DebugCamera {
    camera_position: Point3<f32>,
    orientation: Quaternion<f32>,
    fly_speed: f32,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
}

impl DebugCamera {
    pub fn new() -> Self {
        Self {
            camera_position: Point3::new(0.0, 50.0, 0.0),
            orientation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            fly_speed: 100.0,
            view_matrix: Matrix4::zero(),
            projection_matrix: Matrix4::zero(),
            view_projection_matrix: Matrix4::zero(),
        }
    }

    pub fn look_around(&mut self, mouse_delta: Vector2<f32>) {
        let pitch = Quaternion::from_axis_angle(Vector3::unit_x(), Rad(-mouse_delta.y * LOOK_AROUND_SPEED));
        let yaw = Quaternion::from_axis_angle(Vector3::unit_y(), Rad(-mouse_delta.x * LOOK_AROUND_SPEED));
        self.orientation = (yaw * self.orientation * pitch).normalize();
    }

    pub fn move_forward(&mut self, delta_time: f32) {
        self.camera_position += self.view_direction() * self.fly_speed * delta_time;
    }

    pub fn move_backward(&mut self, delta_time: f32) {
        self.camera_position -= self.view_direction() * self.fly_speed * delta_time;
    }

    pub fn move_left(&mut self, delta_time: f32) {
        let left = self.view_direction().cross(LOOK_UP_VECTOR).normalize();
        self.camera_position += left * self.fly_speed * delta_time;
    }

    pub fn move_right(&mut self, delta_time: f32) {
        let right = LOOK_UP_VECTOR.cross(self.view_direction()).normalize();
        self.camera_position += right * self.fly_speed * delta_time;
    }

    pub fn move_up(&mut self, delta_time: f32) {
        self.camera_position += Vector3::unit_y() * self.fly_speed * delta_time;
    }

    pub fn accelerate(&mut self) {
        self.fly_speed = FLY_SPEED_FAST;
    }

    pub fn decelerate(&mut self) {
        self.fly_speed = FLY_SPEED_SLOW;
    }
}

impl Camera for DebugCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        self.camera_position + self.view_direction()
    }

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        let aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.view_matrix = Matrix4::look_to_lh(self.camera_position, self.view_direction(), LOOK_UP_VECTOR);
        self.projection_matrix = perspective_reverse_lh(VERTICAL_FOV, aspect_ratio);
        self.view_projection_matrix = self.projection_matrix * self.view_matrix;
    }

    fn look_up_vector(&self) -> Vector3<f32> {
        LOOK_UP_VECTOR
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)
    }

    fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.view_projection_matrix
    }

    fn view_direction(&self) -> Vector3<f32> {
        self.orientation.rotate_vector(Vector3::unit_z())
    }
}
