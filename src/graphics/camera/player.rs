use cgmath::{ Matrix4, Vector4, Vector3, Vector2, Point3, Rad, InnerSpace, SquareMatrix, Array };
use graphics::Transform;

use super::{ Camera, SmoothedValue };

const ZOOM_SPEED: f32 = 4.0;
const ROTATION_SPEED: f32 = 0.02;

pub struct PlayerCamera {
    focus_position: Point3<f32>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    world_to_screen_matrix: Matrix4<f32>,
    screen_to_world_matrix: Matrix4<f32>,
    view_angle: SmoothedValue,
    zoom: SmoothedValue,
}

impl PlayerCamera {

    pub fn new() -> Self {
        Self {
            focus_position: Point3::new(0.0, 0.0, 0.0),
            look_up_vector: Vector3::new(0.0, -1.0, 0.0),
            view_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            projection_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            world_to_screen_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            screen_to_world_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            view_angle: SmoothedValue::new(0.0, 0.01, 15.0),
            zoom: SmoothedValue::new(30.0, 0.01, 5.0),
        }
    }

    pub fn set_focus(&mut self, position: Vector3<f32>) {
        self.focus_position = Point3::new(position.x, position.y, position.z);
    }

    pub fn soft_zoom(&mut self, zoom_factor: f32) {
        self.zoom.move_desired(zoom_factor * ZOOM_SPEED);
        // clamp selection
    }

    pub fn soft_rotate(&mut self, rotation: f32) {
        self.view_angle.move_desired(rotation * ROTATION_SPEED);
    }

    pub fn update(&mut self, delta_time: f64) {
        self.zoom.update(delta_time);
        self.view_angle.update(delta_time);
    }

    fn camera_position(&self) -> Point3<f32> {
        let zoom = self.zoom.get_current();
        let view_angle = self.view_angle.get_current();
        return Point3::new(self.focus_position.x + zoom * view_angle.cos(), self.focus_position.y + zoom, self.focus_position.z + -zoom * view_angle.sin());
    }

    fn view_direction(&self) -> Vector3<f32> {
        let camera_position = self.camera_position();
        return Vector3::new(self.focus_position.x - camera_position.x, self.focus_position.y - camera_position.y, self.focus_position.z - camera_position.z).normalize();
    }

    fn world_to_clip_space(&self, world_space_position: Vector3<f32>) -> Vector4<f32> {
        let position = Vector4::new(world_space_position.x, world_space_position.y, world_space_position.z, 1.0);
        return self.world_to_screen_matrix * position;
    }

    fn clip_to_screen_space(&self, clip_space_position: Vector4<f32>) -> Vector2<f32> {
        return Vector2::new(clip_space_position.x / clip_space_position.w + 1.0, clip_space_position.y / clip_space_position.w + 1.0);
    }
}

impl Camera for PlayerCamera {

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        let aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.projection_matrix = cgmath::perspective(Rad(0.2617), aspect_ratio, 1.0, 10000.0);

        let camera_position = self.camera_position();
        self.view_matrix = Matrix4::look_at_rh(camera_position, self.focus_position, self.look_up_vector);

        self.world_to_screen_matrix = self.projection_matrix * self.view_matrix;
        self.screen_to_world_matrix = self.world_to_screen_matrix.invert().unwrap();
    }

    fn transform_matrices(&self, transform: &Transform) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
        let translation_matrix = Matrix4::from_translation(transform.position);
        let rotation_matrix = Matrix4::from_angle_x(transform.rotation.x) * Matrix4::from_angle_y(transform.rotation.y) * Matrix4::from_angle_z(transform.rotation.z);
        let scale_matrix = Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);
        let world_matrix = rotation_matrix * translation_matrix * scale_matrix;

        return (rotation_matrix, world_matrix, self.view_matrix, self.projection_matrix);
    }

    fn billboard_coordinates(&self, position: Vector3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>) {

        let view_direction = self.view_direction();
        let right_vector = self.look_up_vector.cross(view_direction).normalize();
        let up_vector = view_direction.cross(right_vector).normalize();

        let top_left_position = self.world_to_clip_space(position + (up_vector - right_vector) * size);
        let bottom_right_position = self.world_to_clip_space(position + (right_vector - up_vector) * size);

        return (top_left_position, bottom_right_position);
    }

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (Vector2<f32>, Vector2<f32>) {
        let top_left_position = self.clip_to_screen_space(top_left_position);
        let bottom_right_position = self.clip_to_screen_space(bottom_right_position);

        let screen_position = top_left_position;
        let screen_size = bottom_right_position - top_left_position;

        return (screen_position, screen_size);
    }

    fn distance_to(&self, position: Vector3<f32>) -> f32 {
        let delta = self.camera_position() - position;
        return delta.map(|component| component * component).sum().sqrt();
    }

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32> {
        return self.screen_to_world_matrix;
    }
}
