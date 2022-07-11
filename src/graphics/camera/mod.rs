mod player;
#[cfg(feature = "debug")]
mod debug;

use std::sync::Arc;

pub use self::player::PlayerCamera;
#[cfg(feature = "debug")]
pub use self::debug::DebugCamera;

use cgmath::{ Matrix4, Vector4, Vector3, Vector2 };
use crate::graphics::{ SmoothedValue, Transform };
use crate::types::Entity;
use crate::types::map::Map;
use super::RenderSettings;

pub trait Camera {

    fn generate_view_projection(&mut self, window_size: Vector2<usize>);

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>);

    fn transform_matrix(&self, transform: &Transform) -> Matrix4<f32>;

    fn billboard_matrix(&self, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>) -> Matrix4<f32>;

    fn billboard_coordinates(&self, position: Vector3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>);

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (Vector2<f32>, Vector2<f32>);

    fn distance_to(&self, position: Vector3<f32>) -> f32;

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32>;

    fn get_light_matrix(&self) -> Matrix4<f32>;

    //fn render_scene(&self, map: Arc<Map>, entities: Arc<Vec<Arc<Entity>>>, render_settings: &RenderSettings, client_tick: u32);
}
