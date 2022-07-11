mod node;
mod shading;

use derive_new::new;
use crate::graphics::{ Renderer, Camera, Transform };
use crate::loaders::ModelData;

pub use self::node::{ Node, BoundingBox };
pub use self::shading::ShadingType;

#[derive(PrototypeElement, new)]
pub struct Model {
    pub root_node: Node,
    #[cfg(feature = "debug")]
    pub model_data: ModelData,
    #[cfg(feature = "debug")]
    pub bounding_box: BoundingBox,
}

impl Model {

    pub fn render_geometry(&self, renderer: &mut Renderer, camera: &dyn Camera, root_transform: &Transform, client_tick: u32) {
        self.root_node.render_geometry(renderer, camera, root_transform, client_tick);
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(&self, renderer: &mut Renderer, camera: &dyn Camera, root_transform: &Transform) {
        renderer.render_bounding_box(camera, &root_transform, &self.bounding_box);
    }
}
