use std::f32::consts::FRAC_PI_4;

use cgmath::{Array, EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Rad, SquareMatrix, Vector2, Vector3, Vector4};
use ragnarok_formats::transform::Transform;

use super::Camera;
use crate::interface::layout::{ScreenPosition, ScreenSize};

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
    const FAR_PLANE: f32 = 10000.0;
    const NEAR_PLANE: f32 = 0.5;

    pub fn new() -> Self {
        Self {
            camera_position: Point3::new(0.0, 10.0, 0.0),
            look_up_vector: Vector3::new(0.0, -1.0, 0.0),
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
        let forward_vector = self.focus_position() - self.camera_position;
        self.camera_position += forward_vector * self.fly_speed * delta_time;
    }

    pub fn move_backward(&mut self, delta_time: f32) {
        let forward_vector = self.focus_position() - self.camera_position;
        self.camera_position -= forward_vector * self.fly_speed * delta_time;
    }

    pub fn move_left(&mut self, delta_time: f32) {
        let forward_vector = self.focus_position() - self.camera_position;
        self.camera_position += self.look_up_vector.cross(forward_vector) * self.fly_speed * delta_time;
    }

    pub fn move_right(&mut self, delta_time: f32) {
        let forward_vector = self.focus_position() - self.camera_position;
        self.camera_position -= self.look_up_vector.cross(forward_vector) * self.fly_speed * delta_time;
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

    fn focus_position(&self) -> Point3<f32> {
        let x = self.yaw.0.cos() * self.pitch.0.cos();
        let y = self.pitch.0.sin();
        let z = self.yaw.0.sin() * self.pitch.0.cos();

        Point3::new(
            self.camera_position.x + x,
            self.camera_position.y + y,
            self.camera_position.z + z,
        )
    }

    fn view_direction(&self) -> Vector3<f32> {
        let focus_position = self.focus_position();
        Vector3::new(
            focus_position.x - self.camera_position.x,
            focus_position.y - self.camera_position.y,
            focus_position.z - self.camera_position.z,
        )
        .normalize()
    }

    fn world_to_clip_space(&self, world_space_position: Vector3<f32>) -> Vector4<f32> {
        let position = Vector4::new(world_space_position.x, world_space_position.y, world_space_position.z, 1.0);
        self.world_to_screen_matrix * position
    }

    fn clip_to_screen_space(&self, clip_space_position: Vector4<f32>) -> Vector2<f32> {
        Vector2::new(
            clip_space_position.x / clip_space_position.w + 1.0,
            clip_space_position.y / clip_space_position.w + 1.0,
        )
    }
}

impl Camera for DebugCamera {
    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        let aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.projection_matrix = cgmath::perspective(Rad(FRAC_PI_4), aspect_ratio, 0.5, 10000.0);
        self.view_matrix = Matrix4::look_at_rh(self.camera_position, self.focus_position(), self.look_up_vector);

        self.world_to_screen_matrix = self.projection_matrix * self.view_matrix;
        self.screen_to_world_matrix = self.world_to_screen_matrix.invert().unwrap();
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)
    }

    fn transform_matrix(&self, transform: &Transform) -> Matrix4<f32> {
        let translation_matrix = Matrix4::from_translation(transform.position);
        let rotation_matrix = Matrix4::from_angle_x(transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y)
            * Matrix4::from_angle_z(transform.rotation.z);
        let scale_matrix = Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);

        translation_matrix * rotation_matrix * scale_matrix
    }

    fn billboard_matrix(&self, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>) -> Matrix4<f32> {
        let direction = self.view_direction();
        let right_vector = self.look_up_vector.cross(direction).normalize();
        let up_vector = direction.cross(right_vector).normalize();

        let rotation_matrix = Matrix4::from_cols(
            right_vector.extend(0.0),
            up_vector.extend(0.0),
            direction.extend(0.0),
            Vector3::from_value(0.0).extend(1.0),
        );

        let translation_matrix = Matrix4::from_translation(position);
        let origin_matrix = Matrix4::from_translation(origin);
        let scale_matrix = Matrix4::from_nonuniform_scale(size.x, size.y, 1.0);

        translation_matrix * (rotation_matrix * origin_matrix) * scale_matrix
    }

    fn billboard_coordinates(&self, position: Vector3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>) {
        let view_direction = self.view_direction();
        let right_vector = self.look_up_vector.cross(view_direction).normalize();
        let up_vector = view_direction.cross(right_vector).normalize();

        let top_left_position = self.world_to_clip_space(position + (up_vector - right_vector) * size);
        let bottom_right_position = self.world_to_clip_space(position + (right_vector - up_vector) * size);

        (top_left_position, bottom_right_position)
    }

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (ScreenPosition, ScreenSize) {
        let top_left_position = self.clip_to_screen_space(top_left_position);
        let bottom_right_position = self.clip_to_screen_space(bottom_right_position);

        let screen_position = ScreenPosition {
            left: top_left_position.x,
            top: top_left_position.y,
        };
        let screen_size = ScreenSize {
            width: bottom_right_position.x - top_left_position.x,
            height: bottom_right_position.y - top_left_position.y,
        };

        (screen_position, screen_size)
    }

    fn distance_to(&self, position: Vector3<f32>) -> f32 {
        self.camera_position.distance(Point3::from_vec(position))
    }

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32> {
        self.screen_to_world_matrix
    }

    fn get_camera_direction(&self) -> usize {
        let view_direction = self.view_direction();
        super::direction(Vector2::new(view_direction.x, view_direction.z))
    }

    fn calculate_depth_offset_and_curvature(&self, world_matrix: &Matrix4<f32>) -> (f32, f32) {
        let zero_point = world_matrix * Vector4::new(0.0, 0.0, 0.0, 1.0);
        let front_point = world_matrix * Vector4::new(0.0, -2.0, 4.0, 1.0);
        let top_point = world_matrix * Vector4::new(0.0, -2.0, 0.0, 1.0);
        let visual_length = zero_point.distance(top_point);
        let visual_top_point = zero_point + Vector4::new(0.0, visual_length, 0.0, 0.0);

        let linear_to_non_linear = |linear_depth: f32| {
            (2.0 * Self::FAR_PLANE * Self::NEAR_PLANE)
                / (Self::FAR_PLANE + Self::NEAR_PLANE - linear_depth * (Self::FAR_PLANE - Self::NEAR_PLANE))
        };

        let front_depth = linear_to_non_linear((self.world_to_screen_matrix * front_point).z);
        let top_depth = linear_to_non_linear((self.world_to_screen_matrix * top_point).z);
        let visual_top_depth = linear_to_non_linear((self.world_to_screen_matrix * visual_top_point).z);

        let curvature = top_depth - front_depth;
        let depth_offset = visual_top_depth - top_depth;

        (depth_offset, curvature)
    }
}
