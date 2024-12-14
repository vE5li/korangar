use cgmath::{Array, Deg, InnerSpace, Matrix4, Point3, Quaternion, Rad, Rotation, Rotation3, Vector2, Vector3, Zero};

use super::{Camera, SmoothedValue, MAXIMUM_CAMERA_DISTANCE, MINIMUM_CAMERA_DISTANCE};
use crate::graphics::perspective_reverse_lh;

const ZOOM_SPEED: f32 = 2.0;
const ROTATION_SPEED: f32 = 0.01;
const DEFAULT_DISTANCE: f32 = 400.0;
const DEFAULT_ANGLE: f32 = 180_f32.to_radians();
const CAMERA_PITCH: Deg<f32> = Deg(-55.0);
const VERTICAL_FOV: Deg<f32> = Deg(15.5);
const THRESHOLD: f32 = 0.01;
const LOOK_UP: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

pub struct PlayerCamera {
    focus_point: Point3<SmoothedValue>,
    camera_position: Point3<f32>,
    view_direction: Vector3<f32>,
    view_angle: SmoothedValue,
    camera_distance: SmoothedValue,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
}

impl PlayerCamera {
    pub fn new() -> Self {
        Self {
            focus_point: [SmoothedValue::new(0.0, THRESHOLD, 5.0); 3].into(),
            camera_position: Point3::from_value(0.0),
            view_direction: Vector3::zero(),
            view_angle: SmoothedValue::new(DEFAULT_ANGLE, THRESHOLD, 15.0),
            camera_distance: SmoothedValue::new(DEFAULT_DISTANCE, THRESHOLD, 5.0),
            view_matrix: Matrix4::zero(),
            projection_matrix: Matrix4::zero(),
            view_projection_matrix: Matrix4::zero(),
        }
    }

    pub fn set_focus_point(&mut self, position: Point3<f32>) {
        self.focus_point.x.set(position.x);
        self.focus_point.y.set(position.y);
        self.focus_point.z.set(position.z);
    }

    pub fn set_smoothed_focus_point(&mut self, position: Point3<f32>) {
        self.focus_point.x.set_desired(position.x);
        self.focus_point.y.set_desired(position.y);
        self.focus_point.z.set_desired(position.z);
    }

    pub fn soft_zoom(&mut self, zoom_factor: f32) {
        self.camera_distance
            .move_desired_clamp(zoom_factor * ZOOM_SPEED, MINIMUM_CAMERA_DISTANCE, MAXIMUM_CAMERA_DISTANCE);
    }

    pub fn soft_rotate(&mut self, rotation: f32) {
        self.view_angle.move_desired(rotation * ROTATION_SPEED);
    }

    pub fn reset_rotation(&mut self) {
        self.view_angle.set_desired(DEFAULT_ANGLE);
    }

    pub fn update(&mut self, delta_time: f64) {
        self.focus_point.x.update(delta_time);
        self.focus_point.y.update(delta_time);
        self.focus_point.z.update(delta_time);
        self.camera_distance.update(delta_time);
        self.view_angle.update(delta_time);

        let view_distance = self.camera_distance.get_current();
        let view_angle = self.view_angle.get_current();

        let pitch_rotation = Quaternion::from_angle_x(CAMERA_PITCH);
        let yaw_rotation = Quaternion::from_angle_y(Rad(view_angle));
        let rotation = yaw_rotation * pitch_rotation;
        let base_offset = Vector3::new(0.0, 0.0, view_distance);
        let rotated_offset = rotation.rotate_vector(base_offset);

        self.camera_position = self.focus_point() + rotated_offset;
        self.view_direction = -rotated_offset.normalize();
    }
}

impl Camera for PlayerCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        self.focus_point.map(|component| component.get_current())
    }

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        let aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.view_matrix = Matrix4::look_to_lh(self.camera_position, self.view_direction, LOOK_UP);
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
