use cgmath::Vector3;
use procedural::{PrototypeElement, PrototypeWindow};
use ragnarok_procedural::ByteConvertable;

#[cfg(feature = "debug")]
use crate::graphics::{Camera, MarkerRenderer, Renderer};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(Clone, PrototypeElement, PrototypeWindow, ByteConvertable)]
#[window_title("Sound Source")]
pub struct SoundSource {
    #[length_hint(80)]
    pub name: String,
    #[length_hint(80)]
    pub sound_file: String,
    pub position: Vector3<f32>,
    pub volume: f32,
    pub width: u32,
    pub height: u32,
    pub range: f32,
    #[version_equals_or_above(2, 0)]
    pub cycle: Option<f32>,
}

impl SoundSource {
    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    pub fn render_marker<T>(
        &self,
        render_target: &mut T::Target,
        renderer: &T,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) where
        T: Renderer + MarkerRenderer,
    {
        renderer.render_marker(render_target, camera, marker_identifier, self.position, hovered);
    }
}
