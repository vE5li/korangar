pub mod model;

use derive_new::new;
use std::sync::Arc;

use crate::graphics::{ Renderer, Camera, Transform };
use crate::types::maths::*;

use self::model::*;

#[derive(PrototypeElement, PrototypeWindow, new)]
#[window_title("object")]
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

    pub fn render_geometry(&self, renderer: &mut Renderer, camera: &dyn Camera, client_tick: u32) {
        self.model.render_geometry(renderer, camera, &self.transform, client_tick);
    }

    //#[cfg(feature = "debug")]
    //pub fn render_node_bounding_boxes(&self, renderer: &mut Renderer, camera: &dyn Camera) {
    //    self.model.render_node_bounding_boxes(renderer, camera, &self.transform);
    //}

    #[cfg(feature = "debug")]
    pub fn hovered(&self, renderer: &Renderer, camera: &dyn Camera, mouse_position: Vector2<f32>, smallest_distance: f32) -> Option<f32> {
        let distance = camera.distance_to(self.transform.position);

        match distance < smallest_distance && renderer.marker_hovered(camera, self.transform.position, mouse_position) {
            true => Some(distance),
            false => None,
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera, hovered: bool) {
        renderer.render_object_marker(camera, self.transform.position, hovered);
    }
}
