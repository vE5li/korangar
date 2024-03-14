#[cfg(feature = "debug")]
mod debug;
mod player;
mod shadow;
mod start;

use cgmath::{InnerSpace, Matrix4, Vector2, Vector3, Vector4};

#[cfg(feature = "debug")]
pub use self::debug::DebugCamera;
pub use self::player::PlayerCamera;
pub use self::shadow::ShadowCamera;
pub use self::start::StartCamera;
use crate::graphics::{SmoothedValue, Transform};
use crate::interface::layout::{ScreenPosition, ScreenSize};

fn direction(vector: Vector2<f32>) -> usize {
    let inverted = false;
    let k = ((f32::atan2(vector.normalize().x, vector.y) * (180.0 / std::f32::consts::PI) + 360.0 - 22.5) / 45.0) as usize;

    match inverted {
        true => (k + 5) & 7,
        false => !k & 7,
    }
}

pub trait Camera {
    fn generate_view_projection(&mut self, window_size: Vector2<usize>);

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>);

    fn transform_matrix(&self, transform: &Transform) -> Matrix4<f32>;

    fn billboard_matrix(&self, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>) -> Matrix4<f32>;

    fn billboard_coordinates(&self, position: Vector3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>);

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (ScreenPosition, ScreenSize);

    fn distance_to(&self, position: Vector3<f32>) -> f32;

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32>;

    fn get_camera_direction(&self) -> usize;

    // TODO: also take the height of the entity
    fn calculate_depth_offset_and_curvature(&self, world_matrix: &Matrix4<f32>) -> (f32, f32);
}
