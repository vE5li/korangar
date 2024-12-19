/// Contains all helper structures that produce the render instructions needed
/// by the graphics engine.
mod effect;
mod game_interface;
mod interface;
#[cfg(feature = "debug")]
mod marker;

use std::sync::Arc;

#[cfg(feature = "debug")]
use cgmath::Point3;
pub use effect::EffectRenderer;
pub use game_interface::{AlignHorizontal, GameInterfaceRenderer};
pub use interface::InterfaceRenderer;
#[cfg(feature = "debug")]
pub use marker::DebugMarkerRenderer;

use crate::graphics::{Color, Texture};
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
#[cfg(feature = "debug")]
use crate::world::Camera;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

/// Trait to render sprite data.
pub trait SpriteRenderer {
    fn render_sprite(
        &self,
        texture: Arc<Texture>,
        position: ScreenPosition,
        size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    );

    fn render_sdf(&self, texture: Arc<Texture>, position: ScreenPosition, size: ScreenSize, screen_clip: ScreenClip, color: Color);
}

/// Trait to render markers.
#[cfg(feature = "debug")]
pub trait MarkerRenderer {
    fn render_marker(&mut self, camera: &dyn Camera, marker_identifier: MarkerIdentifier, position: Point3<f32>, hovered: bool);
}
