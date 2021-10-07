mod smoothed;
mod player;

#[cfg(feature = "debug")]
mod debug;

pub use self::player::PlayerCamera;

#[cfg(feature = "debug")]
pub use self::debug::DebugCamera;

use self::smoothed::SmoothedValue;

use cgmath::{ Matrix4, Vector4, Vector3, Vector2 };
use graphics::Transform;

pub trait Camera {

    fn generate_view_projection(&mut self, window_size: Vector2<usize>);

    fn transform_matrices(&self, transform: &Transform) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>, Matrix4<f32>);

    fn billboard_coordinates(&self, position: Vector3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>);

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (Vector2<f32>, Vector2<f32>);

    fn distance_to(&self, position: Vector3<f32>) -> f32;

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32>;
}
