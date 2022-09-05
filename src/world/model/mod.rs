mod node;
mod shading;

use std::ops::Mul;

use cgmath::{Matrix4, Vector3};
use derive_new::new;
use procedural::*;

pub use self::node::{BoundingBox, Node, OrientedBox};
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

    pub fn bounding_box_matrix(bounding_box: &BoundingBox, transform: &Transform) -> Matrix4<f32> {

        let size = bounding_box.size() / 2.0;
        let scale = size.zip(transform.scale, f32::mul);
        let position = transform.position;

        let offset_matrix = Matrix4::from_translation(Vector3::new(0.0, scale.y, 0.0));

        let rotation_matrix = Matrix4::from_angle_z(-transform.rotation.z)
            * Matrix4::from_angle_x(-transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y);

        Matrix4::from_translation(position) * rotation_matrix * offset_matrix * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z)
    }

    pub fn get_bounding_box_matrix(&self, transform: &Transform) -> Matrix4<f32> {

        let size = self.bounding_box.size() / 2.0;
        let scale = size.zip(transform.scale, f32::mul);
        let position = transform.position;

        let offset_matrix = Matrix4::from_translation(Vector3::new(0.0, scale.y, 0.0));

        let rotation_matrix = Matrix4::from_angle_z(-transform.rotation.z)
            * Matrix4::from_angle_x(-transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y);

        Matrix4::from_translation(position) * rotation_matrix * offset_matrix * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z)
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
