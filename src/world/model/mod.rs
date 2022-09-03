mod node;
mod shading;

use cgmath::Matrix4;
use derive_new::new;
use procedural::*;

pub use self::node::{AxisAlignedBox, BoundingBox, Node};
pub use self::shading::ShadingType;
#[cfg(feature = "debug")]
use crate::graphics::Color;
use crate::graphics::{Camera, DeferredRenderer, GeometryRenderer, Renderer, Transform};
#[cfg(feature = "debug")]
use crate::loaders::ModelData;

#[derive(PrototypeElement, new)]
pub struct Model {
    pub root_node: Node,
    pub bounding_box: BoundingBox,
    #[cfg(feature = "debug")]
    pub model_data: ModelData,
}

impl Model {

    pub fn render_geometry<T>(
        &self,
        render_target: &mut T::Target,
        renderer: &T,
        camera: &dyn Camera,
        root_transform: &Transform,
        client_tick: u32,
    ) where
        T: Renderer + GeometryRenderer,
    {
        self.root_node
            .render_geometry(render_target, renderer, camera, root_transform, client_tick);
    }

    pub fn get_world_matrix(&self, transform: &Transform, client_tick: u32) -> Matrix4<f32> {
        self.root_node.world_matrix(transform, client_tick)
    }

    pub fn get_bounding_box(&self) -> BoundingBox {
        self.bounding_box
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        root_transform: &Transform,
    ) {
        renderer.render_bounding_box(render_target, camera, &root_transform, &self.bounding_box, Color::monochrome(0));
    }
}
