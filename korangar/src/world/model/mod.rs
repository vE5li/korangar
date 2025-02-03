mod node;

use std::ops::Mul;

use cgmath::{EuclideanSpace, Matrix4, Vector3};
use derive_new::new;
use korangar_interface::elements::PrototypeElement;
use korangar_util::collision::AABB;
#[cfg(feature = "debug")]
use ragnarok_formats::model::ModelData;
use ragnarok_formats::transform::Transform;
use ragnarok_packets::ClientTick;

pub use self::node::Node;
#[cfg(feature = "debug")]
use crate::graphics::Color;
#[cfg(feature = "debug")]
use crate::graphics::DebugAabbInstruction;
use crate::graphics::ModelInstruction;
use crate::world::Camera;

#[derive(PrototypeElement, new)]
pub struct Model {
    pub root_nodes: Vec<Node>,
    pub bounding_box: AABB,
    #[cfg(feature = "debug")]
    pub model_data: ModelData,
}

impl Model {
    pub fn render_geometry(
        &self,
        instructions: &mut Vec<ModelInstruction>,
        transform: &Transform,
        client_tick: ClientTick,
        camera: &dyn Camera,
    ) {
        self.root_nodes
            .iter()
            .enumerate()
            .for_each(|(node_index, node)| node.render_geometry(instructions, transform, client_tick, camera, node_index));
    }

    #[cfg(feature = "debug")]
    pub fn bounding_box_matrix(bounding_box: &AABB, transform: &Transform) -> Matrix4<f32> {
        let size = bounding_box.size() / 2.0;
        let scale = size.zip(transform.scale, f32::mul);
        let position = transform.position;

        let offset_matrix = Matrix4::from_translation(Vector3::new(0.0, scale.y, 0.0));

        let rotation_matrix = Matrix4::from_angle_z(-transform.rotation.z)
            * Matrix4::from_angle_x(-transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y);

        Matrix4::from_translation(position.to_vec())
            * rotation_matrix
            * offset_matrix
            * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z)
    }

    pub fn get_bounding_box_matrix(&self, transform: &Transform) -> Matrix4<f32> {
        let size = self.bounding_box.size() / 2.0;
        let scale = size.zip(transform.scale, f32::mul);
        let position = transform.position;

        let offset_matrix = Matrix4::from_translation(Vector3::new(0.0, scale.y, 0.0));

        let rotation_matrix = Matrix4::from_angle_z(-transform.rotation.z)
            * Matrix4::from_angle_x(-transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y);

        Matrix4::from_translation(position.to_vec())
            * rotation_matrix
            * offset_matrix
            * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z)
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(&self, instructions: &mut Vec<DebugAabbInstruction>, root_transform: &Transform, color: Color) {
        let world_matrix = Model::bounding_box_matrix(&self.bounding_box, root_transform);
        instructions.push(DebugAabbInstruction {
            world: world_matrix,
            color,
        });
    }
}
