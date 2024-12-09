use cgmath::{EuclideanSpace, Matrix4, Point3, SquareMatrix, Transform as PointTransform};
use derive_new::new;
use korangar_interface::elements::PrototypeElement;
use ragnarok_formats::model::RotationKeyframeData;
use ragnarok_formats::transform::Transform;
use ragnarok_packets::ClientTick;

use crate::graphics::ModelInstruction;
use crate::world::Camera;

#[derive(PrototypeElement, new)]
pub struct Node {
    #[hidden_element]
    pub transform_matrix: Matrix4<f32>,
    #[hidden_element]
    pub centroid: Point3<f32>,
    pub transparent: bool,
    pub vertex_offset: usize,
    pub vertex_count: usize,
    pub child_nodes: Vec<Node>,
    pub rotation_keyframes: Vec<RotationKeyframeData>,
}

impl Node {
    fn animaton_matrix(&self, client_tick: ClientTick) -> Matrix4<f32> {
        let last_step = self.rotation_keyframes.last().unwrap();
        let animation_tick = client_tick.0 % last_step.frame;

        let mut last_keyframe_index = 0;
        while self.rotation_keyframes[last_keyframe_index + 1].frame < animation_tick {
            last_keyframe_index += 1;
        }

        let last_step = &self.rotation_keyframes[last_keyframe_index];
        let next_step = &self.rotation_keyframes[(last_keyframe_index + 1) % self.rotation_keyframes.len()];

        let total = next_step.frame.saturating_sub(last_step.frame);
        let offset = animation_tick.saturating_sub(last_step.frame);

        let animation_elapsed = (1.0 / total as f32) * offset as f32;
        let current_rotation = last_step.quaternions.nlerp(next_step.quaternions, animation_elapsed);

        current_rotation.into()
    }

    pub fn world_matrix(&self, transform: &Transform, client_tick: ClientTick) -> Matrix4<f32> {
        let animation_rotation_matrix = match self.rotation_keyframes.is_empty() {
            true => Matrix4::identity(),
            false => self.animaton_matrix(client_tick),
        };

        let rotation_matrix = Matrix4::from_angle_z(-transform.rotation.z)
            * Matrix4::from_angle_x(-transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y);

        Matrix4::from_translation(transform.position.to_vec())
            * rotation_matrix
            * Matrix4::from_nonuniform_scale(transform.scale.x, -transform.scale.y, transform.scale.z)
            * self.transform_matrix
            * animation_rotation_matrix
    }

    pub fn render_geometry(
        &self,
        instructions: &mut Vec<ModelInstruction>,
        transform: &Transform,
        client_tick: ClientTick,
        camera: &dyn Camera,
    ) {
        let model_matrix = self.world_matrix(transform, client_tick);
        let position = model_matrix.transform_point(self.centroid);
        let distance = camera.distance_to(position);

        instructions.push(ModelInstruction {
            model_matrix,
            vertex_offset: self.vertex_offset,
            vertex_count: self.vertex_count,
            distance,
            transparent: self.transparent,
        });

        self.child_nodes
            .iter()
            .for_each(|node| node.render_geometry(instructions, transform, client_tick, camera));
    }
}
