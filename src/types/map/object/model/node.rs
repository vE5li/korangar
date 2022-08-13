use procedural::*;
use derive_new::new;
use crate::loaders::RotationKeyframeData;
use crate::types::maths::*;
use crate::graphics::{ Renderer, Camera, ModelVertexBuffer, Texture, Transform, GeometryRenderer };

#[derive(Copy, Clone, Debug, PrototypeElement)]
pub struct BoundingBox {
    pub smallest: Vector3<f32>,
    pub biggest: Vector3<f32>,
}

impl BoundingBox {
    
    pub fn new<'t, T>(vertex_positions: T) -> Self
        where
            T: IntoIterator<Item = Vector3<f32>>,
    {

        let mut smallest: Vector3<f32> = vector3!(f32::MAX);
        let mut biggest: Vector3<f32> = vector3!(-f32::MAX);

        for position in vertex_positions {

            smallest.x = smallest.x.min(position.x);
            smallest.y = smallest.y.min(position.y);
            smallest.z = smallest.z.min(position.z);

            biggest.x = biggest.x.max(position.x);
            biggest.y = biggest.y.max(position.y);
            biggest.z = biggest.z.max(position.z);
        }

        Self { smallest, biggest }
    }

    pub fn uninitialized() -> Self {
        let smallest: Vector3<f32> = vector3!(f32::MAX);
        let biggest: Vector3<f32> = vector3!(-f32::MAX);
        Self { smallest, biggest }
    }

    pub fn size(&self) -> Vector3<f32> {
        self.biggest - self.smallest
    }

    pub fn center(&self) -> Vector3<f32> {
        self.smallest + self.size() / 2.0
    }

    pub fn extend(&mut self, other: &Self) {
        self.biggest = self.biggest.zip(other.biggest, f32::max);
        self.smallest = self.smallest.zip(other.smallest, f32::min);
    }
}

#[derive(PrototypeElement, new)]
pub struct Node {
    #[hidden_element]
    pub transform_matrix: Matrix4<f32>,
    pub vertex_buffer: ModelVertexBuffer,
    pub textures: Vec<Texture>,
    pub child_nodes: Vec<Node>,
    pub rotation_keyframes: Vec<RotationKeyframeData>,
}

impl Node {

    pub fn world_matrix(&self, transform: &Transform, client_tick: u32) -> Matrix4<f32> {

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
            * Matrix4::from_cols(vector4!(1.0, 0.0, 0.0, 0.0), vector4!(0.0, -1.0, 0.0, 0.0), vector4!(0.0, 0.0, 1.0, 0.0), vector4!(0.0, 0.0, 0.0, 1.0))
            * self.transform_matrix
            * animation_rotation_matrix
    }

    pub fn render_geometry<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, transform: &Transform, client_tick: u32)
        where T: Renderer + GeometryRenderer
    {
        renderer.render_geometry(render_target, camera, self.vertex_buffer.clone(), &self.textures, self.world_matrix(transform, client_tick));
        self.child_nodes.iter().for_each(|node| node.render_geometry(render_target, renderer, camera, transform, client_tick));
    }

    fn animaton_matrix(&self, client_tick: u32) -> Matrix4<f32> {

        let last_step = self.rotation_keyframes.last().unwrap();
        let animation_tick = client_tick % last_step.frame;

        let mut last_keyframe_index = 0;
        while self.rotation_keyframes[last_keyframe_index + 1].frame < animation_tick {
            last_keyframe_index += 1;
        }

        let last_step = &self.rotation_keyframes[last_keyframe_index];
        let next_step = &self.rotation_keyframes[(last_keyframe_index + 1) % self.rotation_keyframes.len()];

        let total = next_step.frame - last_step.frame;
        let offset = animation_tick- last_step.frame;

        let animation_elapsed = (1.0 / total as f32) * offset as f32;
        let current_rotation = last_step.quaternions.nlerp(next_step.quaternions, animation_elapsed);

        current_rotation.into()
    }
}
