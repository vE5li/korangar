mod lookup;

use cgmath::Vector3;
use procedural::*;

#[cfg(feature = "debug")]
use crate::graphics::{Camera, MarkerRenderer, Renderer};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(Clone, ByteConvertable, PrototypeElement, PrototypeWindow)]
#[window_title("Effect Source")]
pub struct EffectSource {
    #[length_hint(80)]
    pub name: String,
    pub position: Vector3<f32>,
    pub effect_type: u32, // TODO: fix this
    pub emit_speed: f32,
    pub _param0: f32,
    pub _param1: f32,
    pub _param2: f32,
    pub _param3: f32,
}

impl EffectSource {
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
