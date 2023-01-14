use std::sync::Arc;

use cgmath::{Matrix4, Vector3};
use derive_new::new;
use procedural::*;

use crate::graphics::*;
use crate::network::ClientTick;
use crate::world::*;

#[derive(PrototypeElement, PrototypeWindow, new)]
pub struct Object {
    pub name: Option<String>,
    pub model_name: String,
    pub model: Arc<Model>,
    pub transform: Transform,
}

impl Object {
    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.transform.position += offset;
    }

    pub fn render_geometry<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, client_tick: ClientTick, time: f32)
    where
        T: Renderer + GeometryRenderer,
    {
        self.model
            .render_geometry(render_target, renderer, camera, &self.transform, client_tick, time);
    }

    pub fn get_bounding_box_matrix(&self) -> Matrix4<f32> {
        self.model.get_bounding_box_matrix(&self.transform)
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
    ) {
        self.model.render_bounding_box(render_target, renderer, camera, &self.transform);
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
        renderer.render_marker(render_target, camera, marker_identifier, self.transform.position, hovered);
    }
}
