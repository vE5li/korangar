use cgmath::Vector3;
use procedural::{PrototypeElement, PrototypeWindow};
use ragnarok_procedural::ByteConvertable;

use crate::graphics::*;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(Clone, ByteConvertable, PrototypeElement, PrototypeWindow)]
#[window_title("Light Source")]
pub struct LightSource {
    #[length_hint(80)]
    pub name: String,
    pub position: Vector3<f32>,
    pub color: ColorRGB,
    pub range: f32,
}

impl LightSource {
    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    pub fn render_light(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
    ) {
        renderer.point_light(render_target, camera, self.position, self.color.to_owned().into(), self.range);
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
