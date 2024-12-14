use cgmath::{InnerSpace, Matrix4, Point3, Vector2, Vector3, Zero};

use super::Camera;
use crate::graphics::orthographic_reverse_lh;

const FAR_PLANE: f32 = 500.0;
const NEAR_PLANE: f32 = -500.0;
const MIN_SCALE: f32 = 0.4;
const MAX_BOUNDS: f32 = 300.0;
const LOOK_UP: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

pub struct DirectionalShadowCamera {
    focus_point: Point3<f32>,
    camera_position: Point3<f32>,
    view_direction: Vector3<f32>,
    zoom_scale: f32,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
}

impl DirectionalShadowCamera {
    pub fn new() -> Self {
        Self {
            focus_point: Point3::new(0.0, 0.0, 0.0),
            camera_position: Point3::new(0.0, 0.0, 0.0),
            view_direction: Vector3::zero(),
            zoom_scale: 0.0,
            view_matrix: Matrix4::zero(),
            projection_matrix: Matrix4::zero(),
            view_projection_matrix: Matrix4::zero(),
        }
    }

    pub fn set_focus_point(&mut self, focus_point: Point3<f32>) {
        // We need to snap the camera to a grid, or else shadows get too noisy.
        let grid_size = 25.0;
        self.focus_point = Point3::new(
            (focus_point.x / grid_size).floor() * grid_size,
            (focus_point.y / grid_size).floor() * grid_size,
            (focus_point.z / grid_size).floor() * grid_size,
        );
    }

    // The zoom_scale is used to scale the shadow map and give the best possible
    // resolution for objects shadows.
    pub fn update(&mut self, direction_to_light: Vector3<f32>, zoom_scale: f32) {
        // TODO: NHA Currently the directional light is the direction TO the light.
        //       We should change that to make it the direction the light shines.
        let direction_to_light = direction_to_light.normalize();
        let scaled_direction = direction_to_light * 100.0;
        self.camera_position = self.focus_point + scaled_direction;
        self.view_direction = -direction_to_light;
        self.zoom_scale = zoom_scale;
    }

    fn calculate_bounds(&self) -> f32 {
        let adjusted_scale = MIN_SCALE + (1.0 - MIN_SCALE) * self.zoom_scale;
        MAX_BOUNDS * adjusted_scale
    }
}

impl Camera for DirectionalShadowCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        self.focus_point
    }

    fn generate_view_projection(&mut self, _window_size: Vector2<usize>) {
        let bound_size = self.calculate_bounds();
        self.view_matrix = Matrix4::look_to_lh(self.camera_position(), self.view_direction, LOOK_UP);
        self.projection_matrix = orthographic_reverse_lh(-bound_size, bound_size, -bound_size, bound_size, NEAR_PLANE, FAR_PLANE);
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
