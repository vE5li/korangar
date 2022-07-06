use derive_new::new;
use crate::loaders::RotationKeyframeData;
use crate::types::maths::*;
use crate::graphics::{ Renderer, Camera, ModelVertexBuffer, Texture, Transform };

#[derive(Clone, Debug, PrototypeElement, new)]
pub struct BoundingBox {
    pub smallest: Vector3<f32>,
    pub biggest: Vector3<f32>,
    pub range: Vector3<f32>, // move these to function calls?
}

impl BoundingBox {
    
    pub fn new_new<'t, T>(vertex_positions: T) -> Self
        where
            T: IntoIterator<Item = Vector3<f32>>,
    {

        let mut smallest: Vector3<f32> = vector3!(999999.0);
        let mut biggest: Vector3<f32> = vector3!(-999999.0);

        for position in vertex_positions {

            smallest.x = smallest.x.min(position.x);
            smallest.y = smallest.y.min(position.y);
            smallest.z = smallest.z.min(position.z);

            biggest.x = biggest.x.max(position.x);
            biggest.y = biggest.y.max(position.y);
            biggest.z = biggest.z.max(position.z);
        }

        let range = (biggest - smallest) / 2.0;

        Self { smallest, biggest, range }
    }

    pub fn center(&self) -> Vector3<f32> {
        self.smallest + self.range
    }
}

#[derive(PrototypeElement, new)]
pub struct Node {
    #[hidden_element]
    pub transform_matrix: Matrix4<f32>,
    #[hidden_element]
    pub vertex_buffer: ModelVertexBuffer,
    #[hidden_element]
    pub textures: Vec<Texture>,
    pub child_nodes: Vec<Node>,
    #[hidden_element]
    pub rotation_keyframes: Vec<RotationKeyframeData>,
}

impl Node {

    pub fn render_geometry(&self, renderer: &mut Renderer, camera: &dyn Camera, parent_transform: &Transform, client_tick: u32) {
        renderer.render_node(camera, self, parent_transform, client_tick);
        self.child_nodes.iter().for_each(|node| node.render_geometry(renderer, camera, parent_transform, client_tick));
    }

    pub fn animaton_matrix(&self, client_tick: u32) -> Matrix4<f32> {

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
