use cgmath::{Matrix4, SquareMatrix, Vector4};
use derive_new::new;
use korangar_interface::elements::PrototypeElement;
use ragnarok_formats::model::RotationKeyframeData;
use ragnarok_formats::transform::Transform;
use ragnarok_packets::ClientTick;
use wgpu::RenderPass;

use crate::graphics::{Buffer, Camera, GeometryRenderer, ModelVertex, Renderer, TextureGroup};

#[derive(PrototypeElement, new)]
pub struct Node {
    #[hidden_element]
    pub transform_matrix: Matrix4<f32>,
    #[hidden_element]
    pub vertex_buffer: Buffer<ModelVertex>,
    #[hidden_element]
    pub textures: TextureGroup,
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

        let total = next_step.frame - last_step.frame;
        let offset = animation_tick - last_step.frame;

        let animation_elapsed = (1.0 / total as f32) * offset as f32;
        let current_rotation = last_step.quaternions.nlerp(next_step.quaternions, animation_elapsed);

        current_rotation.into()
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn world_matrix(&self, transform: &Transform, client_tick: ClientTick) -> Matrix4<f32> {
        let animation_rotation_matrix = match self.rotation_keyframes.is_empty() {
            true => Matrix4::identity(),
            false => self.animaton_matrix(client_tick),
        };

        let rotation_matrix = Matrix4::from_angle_z(-transform.rotation.z)
            * Matrix4::from_angle_x(-transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y);

        Matrix4::from_translation(transform.position)
            * rotation_matrix
            * Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z)
            * Matrix4::from_cols(
                Vector4::new(1.0, 0.0, 0.0, 0.0),
                Vector4::new(0.0, -1.0, 0.0, 0.0),
                Vector4::new(0.0, 0.0, 1.0, 0.0),
                Vector4::new(0.0, 0.0, 0.0, 1.0),
            )
            * self.transform_matrix
            * animation_rotation_matrix
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render node geometry"))]
    pub fn render_geometry<T>(
        &self,
        render_target: &mut T::Target,
        render_pass: &mut RenderPass,
        renderer: &T,
        camera: &dyn Camera,
        transform: &Transform,
        client_tick: ClientTick,
        time: f32,
    ) where
        T: Renderer + GeometryRenderer,
    {
        renderer.render_geometry(
            render_target,
            render_pass,
            camera,
            &self.vertex_buffer,
            &self.textures,
            self.world_matrix(transform, client_tick),
            time,
        );

        self.child_nodes
            .iter()
            .for_each(|node| node.render_geometry(render_target, render_pass, renderer, camera, transform, client_tick, time));
    }
}
