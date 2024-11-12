use std::f32::consts::FRAC_PI_2;

use cgmath::{InnerSpace, Matrix4, Point3, Rad, SquareMatrix, Vector2, Vector3};

use super::{Camera, SmoothedValue};
use crate::graphics::perspective_reverse_lh;

const ZOOM_SPEED: f32 = 2.0;
const ROTATION_SPEED: f32 = 0.01;
const MINIMUM_ZOOM: f32 = 150.0;
const MAXIMUM_ZOOM: f32 = 600.0;
const DEFAULT_ZOOM: f32 = 400.0;
const THRESHHOLD: f32 = 0.01;

pub struct PlayerCamera {
    focus_point: Point3<SmoothedValue>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    world_to_screen_matrix: Matrix4<f32>,
    view_angle: SmoothedValue,
    zoom: SmoothedValue,
    aspect_ratio: f32,
}

impl PlayerCamera {
    pub fn new() -> Self {
        Self {
            focus_point: [SmoothedValue::new(0.0, THRESHHOLD, 5.0); 3].into(),
            look_up_vector: Vector3::new(0.0, 1.0, 0.0),
            view_matrix: Matrix4::from_value(0.0),
            projection_matrix: Matrix4::from_value(0.0),
            world_to_screen_matrix: Matrix4::from_value(0.0),
            view_angle: SmoothedValue::new(FRAC_PI_2, THRESHHOLD, 15.0),
            zoom: SmoothedValue::new(DEFAULT_ZOOM, THRESHHOLD, 5.0),
            aspect_ratio: 0.0,
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
        self.zoom.move_desired_clamp(zoom_factor * ZOOM_SPEED, MINIMUM_ZOOM, MAXIMUM_ZOOM);
    }

    pub fn soft_rotate(&mut self, rotation: f32) {
        self.view_angle.move_desired(rotation * ROTATION_SPEED);
    }

    pub fn update(&mut self, delta_time: f64) {
        self.focus_point.x.update(delta_time);
        self.focus_point.y.update(delta_time);
        self.focus_point.z.update(delta_time);
        self.zoom.update(delta_time);
        self.view_angle.update(delta_time);
    }
}

impl Camera for PlayerCamera {
    fn camera_position(&self) -> Point3<f32> {
        let zoom = self.zoom.get_current();
        let view_angle = self.view_angle.get_current();
        Point3::new(
            self.focus_point.x.get_current() + zoom * view_angle.cos(),
            self.focus_point.y.get_current() + zoom,
            self.focus_point.z.get_current() + -zoom * view_angle.sin(),
        )
    }

    fn focus_point(&self) -> Point3<f32> {
        self.focus_point.map(|component| component.get_current())
    }

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        self.aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.projection_matrix = perspective_reverse_lh(Rad(0.2617), self.aspect_ratio);

        let camera_position = self.camera_position();
        self.view_matrix = Matrix4::look_at_lh(camera_position, self.focus_point(), self.look_up_vector);

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

    fn view_direction(&self) -> Vector3<f32> {
        let camera_position = self.camera_position();
        Vector3::new(
            self.focus_point.x.get_current() - camera_position.x,
            self.focus_point.y.get_current() - camera_position.y,
            self.focus_point.z.get_current() - camera_position.z,
        )
        .normalize()
    }
}
