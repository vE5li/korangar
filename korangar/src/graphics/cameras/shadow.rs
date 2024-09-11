use std::f32::consts::FRAC_PI_2;

use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, SquareMatrix, Vector2, Vector3, Vector4};

use super::{orthographic_lh, Camera};

pub struct ShadowCamera {
    focus_point: Point3<f32>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    world_to_screen_matrix: Matrix4<f32>,
    screen_to_world_matrix: Matrix4<f32>,
    day_timer: f32,
}

impl ShadowCamera {
    const FAR_PLANE: f32 = 500.0;
    const NEAR_PLANE: f32 = -1000.0;

    pub fn new() -> Self {
        Self {
            focus_point: Point3::new(0.0, 0.0, 0.0),
            look_up_vector: Vector3::new(0.0, 1.0, 0.0),
            view_matrix: Matrix4::from_value(0.0),
            projection_matrix: Matrix4::from_value(0.0),
            world_to_screen_matrix: Matrix4::from_value(0.0),
            screen_to_world_matrix: Matrix4::from_value(0.0),
            day_timer: 0.0,
        }
    }

    pub fn set_focus_point(&mut self, focus_point: Point3<f32>) {
        self.focus_point = focus_point;
    }

    pub fn update(&mut self, day_timer: f32) {
        self.day_timer = day_timer;
    }
}

impl Camera for ShadowCamera {
    fn camera_position(&self) -> Point3<f32> {
        let direction = crate::world::get_light_direction(self.day_timer).normalize();
        let scaled_direction = direction * 100.0;
        self.focus_point + scaled_direction
    }

    fn focus_point(&self) -> Point3<f32> {
        self.focus_point
    }

    fn generate_view_projection(&mut self, _window_size: Vector2<usize>) {
        let bounds = Vector4::new(-300.0, 300.0, -300.0, 300.0);

        self.projection_matrix = orthographic_lh(bounds.x, bounds.y, bounds.w, bounds.z, Self::NEAR_PLANE, Self::FAR_PLANE);
        self.view_matrix = Matrix4::look_at_lh(self.camera_position(), self.focus_point, self.look_up_vector);
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

    fn calculate_depth_offset_and_curvature(&self, world_matrix: &Matrix4<f32>, sprite_height: f32, sprite_width: f32) -> (f32, f32) {
        const OFFSET_FACTOR: f32 = 10.0;
        const CURVATURE_FACTOR: f32 = 8.0;

        let sprite_height = -2.0 * sprite_height;

        let sprite_position = world_matrix * Vector4::new(0.0, 0.0, 0.0, 1.0);
        let camera_position = self.camera_position().to_vec().extend(1.0);
        let view_direction = self.view_direction().extend(0.0);

        // Calculate angle from the camera to the sprite in against the x/z plane.
        let camera_to_sprite = (sprite_position - camera_position).normalize();
        let vertical_axis = Vector4::unit_y();
        let sprite_angle = camera_to_sprite.angle(vertical_axis).0;

        // Adjust the angle to make 0.0 degrees the horizon.
        let sprite_angle = (sprite_angle - FRAC_PI_2).to_degrees();
        let angle_progress = sprite_angle / -90.0;

        // Calculate offset point in the opposite view direction.
        let offset_magnitude = OFFSET_FACTOR * sprite_height * angle_progress;
        let offset_point = sprite_position - view_direction * offset_magnitude;

        // Calculate linear depth offset in view space.
        let (view_matrix, _) = self.view_projection_matrices();
        let sprite_view = view_matrix * sprite_position;
        let offset_view = view_matrix * offset_point;
        let depth_offset = offset_view.z - sprite_view.z;

        let curvature = CURVATURE_FACTOR * sprite_width;

        (depth_offset, curvature)
    }
}
