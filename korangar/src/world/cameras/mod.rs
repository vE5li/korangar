#[cfg(feature = "debug")]
mod debug;
mod directional_shadow;
mod player;
mod point_shadow;
pub mod smoothed;
mod start;

use std::f32::consts::FRAC_PI_2;

use cgmath::{Array, EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Vector2, Vector3, Vector4};

#[cfg(feature = "debug")]
pub use self::debug::DebugCamera;
pub use self::directional_shadow::DirectionalShadowCamera;
pub use self::player::PlayerCamera;
pub use self::point_shadow::PointShadowCamera;
pub use self::smoothed::SmoothedValue;
pub use self::start::StartCamera;
#[cfg(feature = "debug")]
use crate::interface::layout::{ScreenPosition, ScreenSize};

/// The world space has a left-handed coordinate system where the Y axis is up.
///
/// +X is right.
/// +Y is up.
/// +Z is into the screen.
pub trait Camera {
    fn camera_position(&self) -> Point3<f32>;
    fn focus_point(&self) -> Point3<f32>;
    fn generate_view_projection(&mut self, window_size: Vector2<usize>);
    fn look_up_vector(&self) -> Vector3<f32>;
    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>);

    #[cfg(feature = "debug")]
    fn world_to_screen_matrix(&self) -> Matrix4<f32>;

    fn billboard_matrix(&self, position: Point3<f32>, origin: Point3<f32>, size: Vector2<f32>) -> Matrix4<f32> {
        let view_direction = self.view_direction();
        let right_vector = self.look_up_vector().cross(view_direction).normalize();
        let up_vector = view_direction.cross(right_vector).normalize();

        let rotation_matrix = Matrix4::from_cols(
            right_vector.extend(0.0),
            up_vector.extend(0.0),
            view_direction.extend(0.0),
            Vector3::from_value(0.0).extend(1.0),
        );

        let translation_matrix = Matrix4::from_translation(position.to_vec());
        let origin_matrix = Matrix4::from_translation(-origin.to_vec());
        let scale_matrix = Matrix4::from_nonuniform_scale(size.x, size.y, 1.0);

        translation_matrix * (rotation_matrix * origin_matrix) * scale_matrix
    }

    #[cfg(feature = "debug")]
    fn billboard_coordinates(&self, position: Point3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>) {
        let view_direction = self.view_direction();
        let right_vector = self.look_up_vector().cross(view_direction).normalize();
        let up_vector = view_direction.cross(right_vector).normalize();

        let world_to_screen_matrix = self.world_to_screen_matrix();

        let top_left_vector = up_vector - right_vector;
        let bottom_right_vector = right_vector - up_vector;

        let top_left_position = world_to_screen_matrix * (position + top_left_vector * size).to_homogeneous();
        let bottom_right_position = world_to_screen_matrix * (position + bottom_right_vector * size).to_homogeneous();

        (top_left_position, bottom_right_position)
    }

    fn calculate_depth_offset_and_curvature(&self, world_matrix: &Matrix4<f32>, sprite_height: f32, sprite_width: f32) -> (f32, f32) {
        const OFFSET_FACTOR: f32 = 10.0;
        const CURVATURE_FACTOR: f32 = 8.0;

        let sprite_height = 2.0 * sprite_height;

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

    fn camera_direction(&self) -> usize {
        let view_direction = self.view_direction();
        direction(Vector2::new(view_direction.x, view_direction.z))
    }

    /// Converts a clip space location (NDC) into screen space coordinates (UV).
    ///                 NDC          UV
    /// Top Left       -1,1         0,0
    /// Bottom Right   1,-1         1,1
    fn clip_to_screen_space(&self, clip_space_position: Vector4<f32>) -> Vector2<f32> {
        let x = clip_space_position.x / clip_space_position.w;
        let y = clip_space_position.y / clip_space_position.w;
        Vector2::new((x + 1.0) * 0.5, (1.0 - y) * 0.5)
    }

    /// Converts screen space coordinates (UV) into a clip space location (NDC).
    ///                 UV          NDC
    /// Top Left       0,0         -1,1
    /// Bottom Right   1,1         1,-1
    fn screen_to_clip_space(&self, screen_space_position: Vector2<f32>) -> Vector4<f32> {
        let x = screen_space_position.x * 2.0 - 1.0;
        let y = -(screen_space_position.y * 2.0 - 1.0);
        Vector4::new(x, y, 0.0, 1.0)
    }

    fn distance_to(&self, position: Point3<f32>) -> f32 {
        self.camera_position().distance(position)
    }

    #[cfg(feature = "debug")]
    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (ScreenPosition, ScreenSize) {
        let top_left_position = self.clip_to_screen_space(top_left_position);
        let bottom_right_position = self.clip_to_screen_space(bottom_right_position);

        let screen_position = ScreenPosition {
            left: top_left_position.x,
            top: top_left_position.y,
        };

        let screen_size = ScreenSize {
            width: (bottom_right_position.x - top_left_position.x).abs(),
            height: (top_left_position.y - bottom_right_position.y).abs(),
        };

        (screen_position, screen_size)
    }

    fn view_direction(&self) -> Vector3<f32> {
        let focus_position = self.focus_point().to_vec();
        let camera_position = self.camera_position().to_vec();
        (focus_position - camera_position).normalize()
    }
}

fn direction(vector: Vector2<f32>) -> usize {
    let inverted = false;
    let k = ((f32::atan2(vector.normalize().x, vector.y) * (180.0 / std::f32::consts::PI) + 360.0 - 22.5) / 45.0) as usize;

    match inverted {
        true => (k + 5) & 7,
        false => !k & 7,
    }
}

#[cfg(test)]
mod conversion {
    use cgmath::{assert_relative_eq, Vector4};

    use crate::world::{Camera, PlayerCamera};

    #[test]
    fn clip_to_screen_space() {
        let camera = PlayerCamera::new();

        let original = Vector4::new(0.5, -0.3, 0.0, 1.0);
        let screen_space = camera.clip_to_screen_space(original);
        let converted = camera.screen_to_clip_space(screen_space);

        assert_relative_eq!(original.x, converted.x, epsilon = 1e-6);
        assert_relative_eq!(original.y, converted.y, epsilon = 1e-6);
        assert_relative_eq!(original.z, converted.z, epsilon = 1e-6);
        assert_relative_eq!(original.w, converted.w, epsilon = 1e-6);
    }
}
