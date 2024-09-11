use std::f32::consts::FRAC_PI_4;

use cgmath::{Matrix4, Point3, Rad, SquareMatrix, Vector2, Vector3};

use super::{perspective_reverse_lh, Camera};

const LOOK_AROUND_SPEED: f32 = 0.005;
const FLY_SPEED_FAST: f32 = 1000.0;
const FLY_SPEED_SLOW: f32 = 100.0;

pub struct DebugCamera {
    camera_position: Point3<f32>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    world_to_screen_matrix: Matrix4<f32>,
    screen_to_world_matrix: Matrix4<f32>,
    pitch: Rad<f32>,
    yaw: Rad<f32>,
    fly_speed: f32,
}

impl DebugCamera {
    pub fn new() -> Self {
        Self {
            camera_position: Point3::new(0.0, 10.0, 0.0),
            look_up_vector: Vector3::new(0.0, 1.0, 0.0),
            view_matrix: Matrix4::from_value(0.0),
            projection_matrix: Matrix4::from_value(0.0),
            world_to_screen_matrix: Matrix4::from_value(0.0),
            screen_to_world_matrix: Matrix4::from_value(0.0),
            pitch: Rad(0.0),
            yaw: Rad(0.0),
            fly_speed: 100.0,
        }
    }

    pub fn look_around(&mut self, mouse_delta: Vector2<f32>) {
        self.pitch += Rad(mouse_delta.y * LOOK_AROUND_SPEED);
        self.yaw += Rad(mouse_delta.x * LOOK_AROUND_SPEED);
    }

    pub fn move_forward(&mut self, delta_time: f32) {
        let forward_vector = self.focus_point() - self.camera_position;
        self.camera_position += forward_vector * self.fly_speed * delta_time;
    }

    pub fn move_backward(&mut self, delta_time: f32) {
        let forward_vector = self.focus_point() - self.camera_position;
        self.camera_position -= forward_vector * self.fly_speed * delta_time;
    }

    pub fn move_left(&mut self, delta_time: f32) {
        let forward_vector = self.focus_point() - self.camera_position;
        self.camera_position += forward_vector.cross(self.look_up_vector) * self.fly_speed * delta_time;
    }

    pub fn move_right(&mut self, delta_time: f32) {
        let forward_vector = self.focus_point() - self.camera_position;
        self.camera_position -= forward_vector.cross(self.look_up_vector) * self.fly_speed * delta_time;
    }

    pub fn move_up(&mut self, delta_time: f32) {
        self.camera_position += Vector3::new(0.0, 1.0, 0.0) * self.fly_speed * delta_time;
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
        let x = self.yaw.0.cos() * self.pitch.0.cos();
        let y = self.pitch.0.sin();
        let z = self.yaw.0.sin() * self.pitch.0.cos();

        Point3::new(
            self.camera_position.x + x,
            self.camera_position.y + y,
            self.camera_position.z + z,
        )
    }

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        let aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.projection_matrix = perspective_reverse_lh(Rad(FRAC_PI_4), aspect_ratio);
        self.view_matrix = Matrix4::look_at_lh(self.camera_position, self.focus_point(), self.look_up_vector);

        self.world_to_screen_matrix = self.projection_matrix * self.view_matrix;
        self.screen_to_world_matrix = self.world_to_screen_matrix.invert().unwrap();
    }

    fn look_up_vector(&self) -> Vector3<f32> {
        self.look_up_vector
    }

    fn screen_to_world_matrix(&self) -> Matrix4<f32> {
        self.screen_to_world_matrix
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)
    }

    fn world_to_screen_matrix(&self) -> Matrix4<f32> {
        self.world_to_screen_matrix
    }
}
