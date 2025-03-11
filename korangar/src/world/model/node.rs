use cgmath::{Matrix, Matrix4, Point3, SquareMatrix, Transform as PointTransform, Vector4, VectorSpace};
use derive_new::new;
use korangar_interface::elements::PrototypeElement;
use ragnarok_formats::model::{RotationKeyframeData, ScaleKeyframeData, TranslationKeyframeData};
use ragnarok_formats::version::InternalVersion;
use ragnarok_packets::ClientTick;

use crate::graphics::ModelInstruction;
use crate::world::Camera;

#[derive(PrototypeElement, new)]
pub struct Node {
    pub version: InternalVersion,
    #[hidden_element]
    pub transform_matrix: Matrix4<f32>,
    #[hidden_element]
    pub rotation_matrix: Matrix4<f32>,
    #[hidden_element]
    pub parent_rotation_matrix: Matrix4<f32>,
    #[hidden_element]
    pub position: Vector4<f32>,
    #[hidden_element]
    pub centroid: Point3<f32>,
    pub sub_meshes: Vec<SubMesh>,
    pub child_nodes: Vec<Node>,
    pub animation_length: u32,
    pub scale_keyframes: Vec<ScaleKeyframeData>,
    pub translation_keyframes: Vec<TranslationKeyframeData>,
    pub rotation_keyframes: Vec<RotationKeyframeData>,
}

#[derive(PrototypeElement)]
pub struct SubMesh {
    pub vertex_offset: usize,
    pub vertex_count: usize,
    pub texture_index: i32,
    pub transparent: bool,
}

impl Node {
    fn interpolate_keyframes<T, U, F>(
        keyframes: &[T],
        animation_length: u32,
        client_tick: ClientTick,
        get_frame: impl Fn(&T) -> i32,
        get_value: impl Fn(&T) -> U,
        interpolation_function: F,
    ) -> U
    where
        F: FnOnce(U, U, f32) -> U,
    {
        let animation_length = animation_length.max(1);
        let animation_tick = (client_tick.0 % animation_length) as i32;

        let last_keyframe_index = keyframes
            .binary_search_by(|keyframe| get_frame(keyframe).cmp(&animation_tick))
            .unwrap_or_else(|keyframe_index| {
                // Err(i) returns the index where the searched element could be inserted to
                // retain the sort order. This means, that we haven't reached a
                // new keyframe yet and need to use the previous keyframe, hence
                // the saturating sub.
                keyframe_index.saturating_sub(1)
            });

        let last_step = &keyframes[last_keyframe_index];
        let next_step = &keyframes[(last_keyframe_index + 1) % keyframes.len()];

        let last_frame = get_frame(last_step);
        let next_frame = get_frame(next_step);

        let total = next_frame.saturating_sub(last_frame).max(1);
        let offset = animation_tick.saturating_sub(last_frame).min(total);
        let animation_elapsed = (1.0 / total as f32) * offset as f32;

        interpolation_function(get_value(last_step), get_value(next_step), animation_elapsed)
    }

    fn scale_animation_matrix(&self, client_tick: ClientTick) -> Matrix4<f32> {
        let current_scale = Self::interpolate_keyframes(
            &self.scale_keyframes,
            self.animation_length,
            client_tick,
            |keyframe| keyframe.frame,
            |keyframe| keyframe.scale,
            |a, b, t| a.lerp(b, t),
        );

        Matrix4::from_nonuniform_scale(current_scale.x, current_scale.y, current_scale.z)
    }

    fn translation_animation_vector(&self, client_tick: ClientTick) -> Vector4<f32> {
        let current_translation = Self::interpolate_keyframes(
            &self.translation_keyframes,
            self.animation_length,
            client_tick,
            |keyframe| keyframe.frame,
            |keyframe| keyframe.translation,
            |a, b, t| a.lerp(b, t),
        );

        current_translation.extend(0.0)
    }

    fn rotation_animation_matrix(&self, client_tick: ClientTick) -> Matrix4<f32> {
        let current_rotation = Self::interpolate_keyframes(
            &self.rotation_keyframes,
            self.animation_length,
            client_tick,
            |keyframe| keyframe.frame,
            |keyframe| keyframe.quaternions,
            |a, b, t| a.nlerp(b, t),
        );

        current_rotation.into()
    }

    pub fn world_matrix(
        &self,
        client_tick: ClientTick,
        parent_matrix: &Matrix4<f32>,
        parent_rotation_matrix: &Matrix4<f32>,
        parent_transform_matrix: &Matrix4<f32>,
        parent_vector: &Vector4<f32>,
        is_static: bool,
    ) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
        match is_static {
            true => (parent_matrix * self.transform_matrix, Matrix4::identity(), Matrix4::identity()),
            false => match self.version.smaller(2, 2) {
                true => {
                    let animation_scale_matrix = match self.scale_keyframes.is_empty() {
                        true => Matrix4::identity(),
                        false => self.scale_animation_matrix(client_tick),
                    };
                    let animation_rotation_matrix = match self.rotation_keyframes.is_empty() {
                        true => Matrix4::identity(),
                        false => self.rotation_animation_matrix(client_tick),
                    };
                    (
                        parent_matrix * self.transform_matrix * animation_rotation_matrix * animation_scale_matrix,
                        Matrix4::identity(),
                        Matrix4::identity(),
                    )
                }
                false => {
                    // The idea for RSM2 matrix calculation is similar to what is described here:
                    // https://rathena.org/board/topic/127587-rsm2-file-format/
                    // The difference is that the inverse matrix equals the transpose in rotation
                    // matrices. The offset matrix represents the accumulation of the
                    // rotations applied to a matrix.
                    // Another difference is the precomputation of the prefix multiplication of
                    // rotation matrices.

                    let animation_translate_vector = match self.translation_keyframes.is_empty() {
                        true => self.parent_rotation_matrix.transpose() * (self.position - parent_vector),
                        false => self.translation_animation_vector(client_tick),
                    };

                    let animation_scale_matrix = match self.scale_keyframes.is_empty() {
                        true => Matrix4::identity(),
                        false => self.scale_animation_matrix(client_tick),
                    };

                    let animation_rotation_matrix = match self.rotation_keyframes.is_empty() {
                        true => self.parent_rotation_matrix.transpose() * self.rotation_matrix,
                        false => self.rotation_animation_matrix(client_tick),
                    };

                    let current_rotation_matrix = animation_rotation_matrix * animation_scale_matrix;
                    let prefix_rotation_matrix = parent_rotation_matrix * current_rotation_matrix;

                    // The problem can be modeled as robotic arms.
                    // Each node is a rotary joint and the arm is the distance from one node to
                    // another node.
                    // Rotate the rotary joint at the origin to maintain the correct angle
                    let mut current_arm_matrix = current_rotation_matrix;

                    // Shift from the origin by the vector arm size from the parent
                    // node to the current node. The new origin is now the
                    // position of the prefix arm rotation point
                    current_arm_matrix.w += animation_translate_vector;

                    // Rotate the rotary joint from the prefix arm to the correct angle.
                    let mut arm_matrix = parent_rotation_matrix * current_arm_matrix;

                    // Shift the prefix length to the correct position.
                    arm_matrix.w[0] += parent_transform_matrix.w[0];
                    arm_matrix.w[1] += parent_transform_matrix.w[1];
                    arm_matrix.w[2] += parent_transform_matrix.w[2];

                    (parent_matrix * arm_matrix, arm_matrix, prefix_rotation_matrix)
                }
            },
        }
    }

    pub fn render_geometry(
        &self,
        instructions: &mut Vec<ModelInstruction>,
        client_tick: ClientTick,
        camera: &dyn Camera,
        node_index: usize,
        parent_matrix: &Matrix4<f32>,
        parent_rotation_matrix: &Matrix4<f32>,
        parent_transform_matrix: &Matrix4<f32>,
        parent_vector: &Vector4<f32>,
        is_static: bool,
    ) {
        // Some models have multiple nodes with the same position. This can lead so
        // z-fighting, when we sort the model instructions later with an unstable,
        // non-allocating sort. To remove this z-fighting, we add a very small offset to
        // the nodes, so that they always have the same order from the same view
        // perspective.
        let draw_order_offset = (node_index as f32) * 1.1920929e-4_f32;
        let (model_matrix, transform_matrix, rotation_matrix) = self.world_matrix(
            client_tick,
            parent_matrix,
            parent_rotation_matrix,
            parent_transform_matrix,
            parent_vector,
            is_static,
        );
        let position = model_matrix.transform_point(self.centroid);
        let distance = camera.distance_to(position) + draw_order_offset;

        // When the render_geometry is set as static, the model matrix
        // is already pre-calculated.
        // When the render_geometry is set as dynamic, the model matrix
        // needs to be calculated recursively, because the nodes from the model change
        // positions due to animation motion.
        let parent_matrix = match is_static {
            true => parent_matrix,
            false => match self.version.equals_or_above(2, 2) {
                true => parent_matrix,
                false => &model_matrix,
            },
        };

        self.sub_meshes.iter().for_each(|mesh| {
            instructions.push(ModelInstruction {
                model_matrix,
                vertex_offset: mesh.vertex_offset,
                vertex_count: mesh.vertex_count,
                texture_index: mesh.texture_index,
                distance,
                transparent: mesh.transparent,
            });
        });

        self.child_nodes.iter().enumerate().for_each(|(node_index, node)| {
            node.render_geometry(
                instructions,
                client_tick,
                camera,
                node_index,
                parent_matrix,
                &rotation_matrix,
                &transform_matrix,
                &self.position,
                is_static,
            )
        });
    }
}
