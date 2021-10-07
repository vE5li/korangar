use std::sync::Arc;

use cgmath::Vector3;

use entities::Model;
use graphics::{ Renderer, Camera, Transform };

pub struct Object {
    model: Arc<Model>,
    transform: Transform,
}

impl Object {

    pub fn new(model: Arc<Model>, transform: Transform) -> Self {
        return Self { model, transform };
    }

    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.transform.position += offset;
    }

    pub fn render_geometry(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        self.model.render_geometry(renderer, camera, &self.transform);
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.render_object_icon(camera, self.transform.position);
    }
}
