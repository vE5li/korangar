use cgmath::{ Matrix4, Vector2, Vector3, Point3, Rad, SquareMatrix };
use std::f32::consts::FRAC_PI_2;
use graphics::Transform;

use super::{ Camera, SmoothedValue };

pub struct DebugCamera {
    position: Point3<f32>,
    up: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    pitch: Rad<f32>,
    yaw: Rad<f32>,
    fly_speed: f32,
}

impl DebugCamera {

    pub fn new() -> Self {
        Self {
            position: Point3::new(0.0, 10.0, 0.0),
            up: Vector3::new(0.0, -1.0, 0.0),
            view_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            projection_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            pitch: Rad(0.0),
            yaw: Rad(0.0),
            fly_speed: 100.0,
        }
    }

    pub fn move_forward(&mut self, delta_time: f32) {
        let forward_vector = self.look_at() - self.position;
        self.position += forward_vector * self.fly_speed * delta_time;
    }

    pub fn move_backward(&mut self, delta_time: f32) {
        let forward_vector = self.look_at() - self.position;
        self.position -= forward_vector * self.fly_speed * delta_time;
    }

    pub fn move_left(&mut self, delta_time: f32) {
        let forward_vector = self.look_at() - self.position;
        self.position += self.up.cross(forward_vector) * self.fly_speed * delta_time;
    }

    pub fn move_right(&mut self, delta_time: f32) {
        let forward_vector = self.look_at() - self.position;
        self.position -= self.up.cross(forward_vector) * self.fly_speed * delta_time;
    }

    pub fn move_up(&mut self, delta_time: f32) {
        self.position += Vector3::new(0.0, 1.0, 0.0) * self.fly_speed * delta_time;
    }

    pub fn move_down(&mut self, delta_time: f32) {
        self.position -= Vector3::new(0.0, 1.0, 0.0) * self.fly_speed * delta_time;
    }

    fn look_at(&self) -> Point3<f32> {
        return Point3::new(self.position.x + 1.0, self.position.y, self.position.z);
    }
}

impl Camera for DebugCamera {

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        let aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.projection_matrix = cgmath::perspective(Rad(FRAC_PI_2), aspect_ratio, 0.1, 1000.0);
        self.view_matrix = Matrix4::look_at_rh(self.position, self.look_at(), self.up) * Matrix4::from_scale(1.0);
    }

    fn screen_to_world_matrix(&self) -> Matrix4<f32> {
        return (self.projection_matrix * self.view_matrix).invert().unwrap();
    }

    fn transform_matrices(&self, transform: &Transform) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
        let translation_matrix = Matrix4::from_translation(transform.position);
        let rotation_matrix = Matrix4::from_angle_x(transform.rotation.x) * Matrix4::from_angle_y(transform.rotation.y) * Matrix4::from_angle_z(transform.rotation.z);
        let scale_matrix = Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);
        let world_matrix = rotation_matrix * translation_matrix * scale_matrix;

        return (rotation_matrix, world_matrix, self.view_matrix, self.projection_matrix);
    }
}
