use cgmath::Vector3;
use ragnarok_formats::map::LightSource;
use wgpu::RenderPass;

#[cfg(feature = "debug")]
use crate::graphics::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::{Camera, DeferredRenderer, Renderer};

pub trait LightSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    fn render_light(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
    );

    #[cfg(feature = "debug")]
    fn render_marker<T>(
        &self,
        render_target: &mut T::Target,
        render_pass: &mut RenderPass,
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

    fn render_light(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
    ) {
        renderer.point_light(
            render_target,
            render_pass,
            camera,
            self.position,
            self.color.to_owned().into(),
            self.range,
        );
    }

    #[cfg(feature = "debug")]
    fn render_marker<T>(
        &self,
        render_target: &mut T::Target,
        render_pass: &mut RenderPass,
        renderer: &T,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) where
        T: Renderer + MarkerRenderer,
    {
        renderer.render_marker(render_target, render_pass, camera, marker_identifier, self.position, hovered);
    }
}
