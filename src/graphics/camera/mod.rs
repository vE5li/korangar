mod smoothed;
mod player;

#[cfg(feature = "debug")]
mod debug;

pub use self::player::PlayerCamera;

#[cfg(feature = "debug")]
pub use self::debug::DebugCamera;

use self::smoothed::SmoothedValue;

use cgmath::{ Matrix4, Vector2 };
use graphics::Transform;

pub trait Camera {

    fn generate_view_projection(&mut self, window_size: Vector2<usize>);

    fn screen_to_world_matrix(&self) -> Matrix4<f32>;

    fn transform_matrices(&self, transform: &Transform) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>, Matrix4<f32>);
}
