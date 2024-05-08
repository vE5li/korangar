use cgmath::Vector3;
use ragnarok_formats::map::LightSource;

use crate::graphics::*;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

pub trait LightSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    fn render_light(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, camera: &dyn Camera);

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

impl LightSourceExt for LightSource {
    fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    fn render_light(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, camera: &dyn Camera) {
        renderer.point_light(render_target, camera, self.position, self.color.to_owned().into(), self.range);
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
