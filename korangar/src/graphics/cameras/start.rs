use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use cgmath::{Matrix4, Point3, Rad, SquareMatrix, Vector2, Vector3};

use super::{perspective_reverse_lh, Camera};

const DEFAULT_ZOOM: f32 = 150.0;
const ROTATION_SPEED: f32 = 0.03;

pub struct StartCamera {
    focus_point: Point3<f32>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    world_to_screen_matrix: Matrix4<f32>,
    view_angle: f32,
    zoom: f32,
    aspect_ratio: f32,
}

impl StartCamera {
    pub fn new() -> Self {
        Self {
            focus_point: Point3::new(0.0, 0.0, 0.0),
            look_up_vector: Vector3::new(0.0, 1.0, 0.0),
            view_matrix: Matrix4::from_value(0.0),
            projection_matrix: Matrix4::from_value(0.0),
            world_to_screen_matrix: Matrix4::from_value(0.0),
            view_angle: FRAC_PI_2,
            zoom: DEFAULT_ZOOM,
            aspect_ratio: 0.0,
        }
    }

    pub fn set_focus_point(&mut self, focus_point: Point3<f32>) {
        self.focus_point = focus_point;
    }

    pub fn update(&mut self, delta_time: f64) {
        self.view_angle += delta_time as f32 * ROTATION_SPEED;
    }
}

impl Camera for StartCamera {
    fn camera_position(&self) -> Point3<f32> {
        Point3::new(
            self.focus_point.x + self.zoom * self.view_angle.cos(),
            self.focus_point.y + self.zoom,
            self.focus_point.z - self.zoom * self.view_angle.sin(),
        )
    }

    fn focus_point(&self) -> Point3<f32> {
        self.focus_point
    }

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        self.aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.projection_matrix = perspective_reverse_lh(Rad(FRAC_PI_4), self.aspect_ratio);

        let camera_position = self.camera_position();
        self.view_matrix = Matrix4::look_at_lh(camera_position, self.focus_point, self.look_up_vector);

        self.world_to_screen_matrix = self.projection_matrix * self.view_matrix;
    }

    fn look_up_vector(&self) -> Vector3<f32> {
        self.look_up_vector
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)
    }

    #[cfg(feature = "debug")]
    fn world_to_screen_matrix(&self) -> Matrix4<f32> {
        self.world_to_screen_matrix
    }
}
