mod node;
mod shading;

use derive_new::new;
use crate::graphics::{ Renderer, Camera, Transform, DeferredRenderer, GeometryRenderer };
#[cfg(feature = "debug")]
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

    pub fn render_geometry<T>(&self, render_target: &mut <T as Renderer>::Target, renderer: &T, camera: &dyn Camera, root_transform: &Transform, client_tick: u32)
        where T: Renderer + GeometryRenderer
    {
        self.root_node.render_geometry(render_target, renderer, camera, root_transform, client_tick);
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box<T>(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, camera: &dyn Camera, root_transform: &Transform) {
        //renderer.render_bounding_box(render_target, camera, &root_transform, &self.bounding_box);
    }
}
