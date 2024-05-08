mod lookup;

use cgmath::Vector3;
use ragnarok_formats::map::EffectSource;

#[cfg(feature = "debug")]
use crate::graphics::{Camera, MarkerRenderer, Renderer};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

pub trait EffectSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    #[cfg(feature = "debug")]
    fn render_marker<T>(
        &self,
        render_target: &mut T::Target,
        renderer: &T,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) where
        T: Renderer + MarkerRenderer;
}

impl EffectSourceExt for EffectSource {
    fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    fn render_marker<T>(
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
