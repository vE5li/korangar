mod player;
mod shadow;
#[cfg(feature = "debug")]
mod debug;

pub use self::player::PlayerCamera;
pub use self::shadow::ShadowCamera;
#[cfg(feature = "debug")]
pub use self::debug::DebugCamera;

use crate::types::maths::*;
use crate::graphics::{ SmoothedValue, Transform };
use crate::types::map::model::BoundingBox;

pub trait Camera {

    fn generate_view_projection(&mut self, window_size: Vector2<usize>);

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>);

    fn transform_matrix(&self, transform: &Transform) -> Matrix4<f32>;

    fn billboard_matrix(&self, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>) -> Matrix4<f32>;

    fn billboard_coordinates(&self, position: Vector3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>);

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (Vector2<f32>, Vector2<f32>);

    fn distance_to(&self, position: Vector3<f32>) -> f32;

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32>;
}
