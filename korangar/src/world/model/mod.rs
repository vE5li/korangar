mod node;

use std::ops::Mul;

use cgmath::{EuclideanSpace, Matrix4, SquareMatrix, Vector3, Vector4, Zero};
use korangar_interface::element::StateElement;
use korangar_util::collision::AABB;
#[cfg(feature = "debug")]
use ragnarok_formats::model::ModelData;
use ragnarok_formats::transform::Transform;
use ragnarok_formats::version::InternalVersion;
use rust_state::RustState;

pub use self::node::{Node, SubMesh};
#[cfg(feature = "debug")]
use crate::graphics::Color;
#[cfg(feature = "debug")]
use crate::graphics::DebugAabbInstruction;
use crate::graphics::ModelInstruction;
use crate::world::Camera;

#[derive(RustState, StateElement)]
pub struct Model {
    pub version: InternalVersion,
    pub root_nodes: Vec<Node>,
    pub bounding_box: AABB,
    pub is_static: bool,
    #[cfg(feature = "debug")]
    pub model_data: ModelData,
}

impl Model {
    pub fn new(
        version: InternalVersion,
        root_nodes: Vec<Node>,
        bounding_box: AABB,
        is_static: bool,
        #[cfg(feature = "debug")] model_data: ModelData,
    ) -> Self {
        Self {
            version,
            root_nodes,
            bounding_box,
            is_static,
            #[cfg(feature = "debug")]
            model_data,
        }
    }
}

impl Model {
    pub fn get_model_matrix(&self, transform: &Transform) -> Matrix4<f32> {
        let translation_matrix = Matrix4::from_translation(transform.position.to_vec());
        let rotation_matrix = Matrix4::from_angle_z(-transform.rotation.z)
            * Matrix4::from_angle_x(-transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y);
        let scale_matrix = match self.version.equals_or_above(2, 2) {
            true => Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z),
            false => Matrix4::from_nonuniform_scale(transform.scale.x, -transform.scale.y, transform.scale.z),
        };
        translation_matrix * rotation_matrix * scale_matrix
    }

    pub fn render_geometry(
        &self,
        instructions: &mut Vec<ModelInstruction>,
        transform: &Transform,
        animation_timer_ms: f32,
        camera: &dyn Camera,
    ) {
        self.root_nodes.iter().enumerate().for_each(|(node_index, node)| {
            let model_matrix = &self.get_model_matrix(transform);
            node.render_geometry(
                instructions,
                animation_timer_ms,
                camera,
                node_index,
                model_matrix,
                &Matrix4::identity(),
                &Matrix4::identity(),
                &Vector4::zero(),
                self.is_static,
            )
        });
    }

    pub fn calculate_bounding_box_matrix(bounding_box: &AABB, transform: &Transform) -> Matrix4<f32> {
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

    pub fn calculate_aabb(&self, transform: &Transform) -> AABB {
        // For RSM v2.2+ the bounding box center requires adjustment since it's not
        // at the geometric center of the box. We subtract half the height from the
        // Y-coordinate and normalize the result to unit space (by dividing by
        // half-size).
        let size = self.bounding_box.size() / 2.0;
        let center_shift = match self.version.equals_or_above(2, 2) {
            true => {
                let half_height = self.bounding_box.size().y / 2.0;
                (self.bounding_box.center().to_vec() - Vector3::new(0.0, half_height, 0.0))
                    .zip(size, |value, size| if size != 0.0 { value / size } else { 0.0 })
            }
            false => Vector3::new(0.0, 0.0, 0.0),
        };
        let shift_matrix = Matrix4::from_translation(center_shift);
        let transform = Self::calculate_bounding_box_matrix(&self.bounding_box, transform) * shift_matrix;

        AABB::from_transformation_matrix(transform)
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(&self, instructions: &mut Vec<DebugAabbInstruction>, root_transform: &Transform, color: Color) {
        let world_matrix = Self::calculate_bounding_box_matrix(&self.bounding_box, root_transform);
        instructions.push(DebugAabbInstruction {
            world: world_matrix,
            color,
        });
    }
}
