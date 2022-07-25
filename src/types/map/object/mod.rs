pub mod model;

use procedural::*;
use derive_new::new;
use std::sync::Arc;

use crate::types::maths::*;
use crate::graphics::*;

use self::model::*;

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

    pub fn render_geometry<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, client_tick: u32)
        where T: Renderer + GeometryRenderer
    {
        self.model.render_geometry(render_target, renderer, camera, &self.transform, client_tick);
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, camera: &dyn Camera) {
        //self.model.render_bounding_box(render_target, renderer, camera, &self.transform);
    }

    #[cfg(feature = "debug")]
    pub fn render_marker<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, hovered: bool)
        where T: Renderer + MarkerRenderer
    {
        renderer.render_marker(render_target, camera, self.transform.position, hovered);
    }
}
