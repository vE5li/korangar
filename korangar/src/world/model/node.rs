use std::sync::Arc;

use cgmath::{Array, Matrix4, SquareMatrix, Vector3, Vector4};
use derive_new::new;
use korangar_interface::elements::PrototypeElement;
use ragnarok_networking::ClientTick;
use vulkano::buffer::Subbuffer;
use vulkano::image::view::ImageView;

use crate::graphics::{Camera, GeometryRenderer, ModelVertex, Renderer, Transform};
use crate::loaders::RotationKeyframeData;
use crate::system::multiply_matrix4_and_vector3;

#[derive(Copy, Clone, Debug, PrototypeElement)]
pub struct BoundingBox {
    pub smallest: Vector3<f32>,
    pub biggest: Vector3<f32>,
}

impl BoundingBox {
    pub fn new<T>(vertex_positions: T) -> Self
    where
        T: IntoIterator<Item = Vector3<f32>>,
    {
        let mut smallest: Vector3<f32> = Vector3::from_value(f32::MAX);
        let mut biggest: Vector3<f32> = Vector3::from_value(-f32::MAX);

        for position in vertex_positions {
            smallest = smallest.zip(position, f32::min);
            biggest = biggest.zip(position, f32::max);
        }

        Self { smallest, biggest }
    }

    pub fn uninitialized() -> Self {
        let smallest: Vector3<f32> = Vector3::from_value(f32::MAX);
        let biggest: Vector3<f32> = Vector3::from_value(-f32::MAX);

        Self { smallest, biggest }
    }

    pub fn size(&self) -> Vector3<f32> {
        self.biggest - self.smallest
    }

    pub fn center(&self) -> Vector3<f32> {
        self.smallest + self.size() / 2.0
    }

    pub fn extend(&mut self, other: &Self) {
        self.smallest = self.smallest.zip(other.smallest, f32::min);
        self.biggest = self.biggest.zip(other.biggest, f32::max);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct OrientedBox {
    pub corners: [Vector3<f32>; 8],
}

impl OrientedBox {
    pub fn transform(self, transform_matrix: Matrix4<f32>) -> Self {
        let corners = self.corners.map(|corner| multiply_matrix4_and_vector3(&transform_matrix, corner));
        Self { corners }
    }
}

impl Default for OrientedBox {
    fn default() -> Self {
        // TODO: check if this might just need to be 4 points
        let corners = [
            Vector3::new(-1.0, -1.0, -1.0),
            Vector3::new(-1.0, -1.0, 1.0),
            Vector3::new(-1.0, 1.0, -1.0),
            Vector3::new(-1.0, 1.0, 1.0),
            Vector3::new(1.0, -1.0, -1.0),
            Vector3::new(1.0, -1.0, 1.0),
            Vector3::new(1.0, 1.0, -1.0),
            Vector3::new(1.0, 1.0, 1.0),
        ];

        Self { corners }
    }
}

impl From<BoundingBox> for OrientedBox {
    fn from(bounding_box: BoundingBox) -> Self {
        // TODO: check if this might just need to be 4 points
        let corners = [
            Vector3::new(bounding_box.smallest.x, bounding_box.smallest.y, bounding_box.smallest.z),
            Vector3::new(bounding_box.smallest.x, bounding_box.smallest.y, bounding_box.biggest.z),
            Vector3::new(bounding_box.smallest.x, bounding_box.biggest.y, bounding_box.smallest.z),
            Vector3::new(bounding_box.smallest.x, bounding_box.biggest.y, bounding_box.biggest.z),
            Vector3::new(bounding_box.biggest.x, bounding_box.smallest.y, bounding_box.smallest.z),
            Vector3::new(bounding_box.biggest.x, bounding_box.smallest.y, bounding_box.biggest.z),
            Vector3::new(bounding_box.biggest.x, bounding_box.biggest.y, bounding_box.smallest.z),
            Vector3::new(bounding_box.biggest.x, bounding_box.biggest.y, bounding_box.biggest.z),
        ];

        Self { corners }
    }
}

#[derive(PrototypeElement, new)]
pub struct Node {
    #[hidden_element]
    pub transform_matrix: Matrix4<f32>,
    #[hidden_element]
    pub vertex_buffer: Subbuffer<[ModelVertex]>,
    #[hidden_element]
    pub textures: Vec<Arc<ImageView>>,
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

    #[korangar_debug::profile]
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

    #[korangar_debug::profile("render node geometry")]
    pub fn render_geometry<T>(
        &self,
        render_target: &mut T::Target,
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
            camera,
            self.vertex_buffer.clone(),
            &self.textures,
            self.world_matrix(transform, client_tick),
            time,
        );

        self.child_nodes
            .iter()
            .for_each(|node| node.render_geometry(render_target, renderer, camera, transform, client_tick, time));
    }
}
