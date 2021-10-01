use cgmath::{ Matrix4, Vector2, Vector3, Point3, Rad, SquareMatrix };
use std::f32::consts::FRAC_PI_2;
use graphics::Transform;

use super::{ Camera, SmoothedValue };

pub struct PlayerCamera {
    look_at: Point3<f32>,
    up: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    zoom: SmoothedValue,
    view_angle: SmoothedValue,
}

impl PlayerCamera {

    pub fn new() -> Self {
        Self {
            look_at: Point3::new(0.0, 0.0, 0.0),
            up: Vector3::new(0.0, -1.0, 0.0),
            view_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            projection_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            zoom: SmoothedValue::new(30.0, 0.01, 15.0),
            view_angle: SmoothedValue::new(0.0, 0.01, 5.0),
        }
    }

    pub fn set_focus(&mut self, position: Vector3<f32>) {
        self.look_at = Point3::new(position.x, position.y, position.z);
    }

    pub fn soft_zoom(&mut self, zoom_factor: f32) {
        self.zoom.move_desired(zoom_factor);
        // clamp selection
    }

    pub fn soft_rotate(&mut self, rotation: f32) {
        self.view_angle.move_desired(rotation);
    }

    pub fn update(&mut self, delta_time: f64) {
        self.zoom.update(delta_time);
        self.view_angle.update(delta_time);
    }
}

impl Camera for PlayerCamera {

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        let aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.projection_matrix = cgmath::perspective(Rad(0.2617), aspect_ratio, 1.0, 10000.0);

        let zoom = self.zoom.get_current();
        let view_angle = self.view_angle.get_current();
        let camera = Point3::new(self.look_at.x + zoom * view_angle.cos(), self.look_at.y + zoom, self.look_at.z + -zoom * view_angle.sin());

        self.view_matrix = Matrix4::look_at_rh(camera, self.look_at, self.up) * Matrix4::from_scale(1.0);
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
