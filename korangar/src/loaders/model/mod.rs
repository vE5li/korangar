use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Matrix4, Rad, SquareMatrix, Vector2, Vector3};
use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_util::collision::AABB;
use korangar_util::math::multiply_matrix4_and_vector3;
use korangar_util::FileLoader;
use ragnarok_bytes::{ByteStream, FromBytes};
use ragnarok_formats::model::{ModelData, ModelString, NodeData};
use ragnarok_formats::version::InternalVersion;
use wgpu::{BufferUsages, Device, Queue};

use super::error::LoadError;
use super::FALLBACK_MODEL_FILE;
use crate::graphics::{Buffer, NativeModelVertex, Texture, TextureGroup};
use crate::loaders::{GameFileLoader, TextureLoader};
use crate::world::{Model, Node};

#[derive(new)]
pub struct ModelLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    game_file_loader: Arc<GameFileLoader>,
    #[new(default)]
    cache: HashMap<(String, bool), Arc<Model>>,
}

impl ModelLoader {
    fn add_vertices(
        native_vertices: &mut Vec<NativeModelVertex>,
        vertex_positions: &[Vector3<f32>],
        texture_coordinates: &[Vector2<f32>],
        texture_index: u16,
        reverse_vertices: bool,
        reverse_normal: bool,
    ) {
        let normal = match reverse_normal {
            true => NativeModelVertex::calculate_normal(vertex_positions[0], vertex_positions[1], vertex_positions[2]),
            false => NativeModelVertex::calculate_normal(vertex_positions[2], vertex_positions[1], vertex_positions[0]),
        };

        if reverse_vertices {
            for (vertex_position, texture_coordinates) in vertex_positions.iter().copied().zip(texture_coordinates).rev() {
                native_vertices.push(NativeModelVertex::new(
                    vertex_position,
                    normal,
                    *texture_coordinates,
                    texture_index as i32,
                    0.0, // TODO: actually add wind affinity
                ));
            }
        } else {
            for (vertex_position, texture_coordinates) in vertex_positions.iter().copied().zip(texture_coordinates) {
                native_vertices.push(NativeModelVertex::new(
                    vertex_position,
                    normal,
                    *texture_coordinates,
                    texture_index as i32,
                    0.0, // TODO: actually add wind affinity
                ));
            }
        }
    }

    fn make_vertices(node: &NodeData, main_matrix: &Matrix4<f32>, reverse_order: bool) -> Vec<NativeModelVertex> {
        let mut native_vertices = Vec::new();

        let array: [f32; 3] = node.scale.into();
        let reverse_node_order = array.into_iter().fold(1.0, |a, b| a * b).is_sign_negative();

        if reverse_node_order {
            panic!("this can actually happen");
        }

        for face in &node.faces {
            // collect into tiny vec instead ?
            let vertex_positions: Vec<Vector3<f32>> = face
                .vertex_position_indices
                .iter()
                .copied()
                .map(|index| node.vertex_positions[index as usize])
                .map(|position| multiply_matrix4_and_vector3(main_matrix, position))
                .collect();

            let texture_coordinates: Vec<Vector2<f32>> = face
                .texture_coordinate_indices
                .iter()
                .copied()
                .map(|index| node.texture_coordinates[index as usize].coordinates)
                .collect();

            Self::add_vertices(
                &mut native_vertices,
                &vertex_positions,
                &texture_coordinates,
                face.texture_index,
                reverse_order,
                false,
            );

            if face.two_sided != 0 {
                Self::add_vertices(
                    &mut native_vertices,
                    &vertex_positions,
                    &texture_coordinates,
                    face.texture_index,
                    !reverse_order,
                    true,
                );
            }
        }

        native_vertices
    }

    fn calculate_matrices(node: &NodeData, parent_matrix: &Matrix4<f32>) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
        let main = Matrix4::from_translation(node.translation1) * Matrix4::from(node.offset_matrix);

        let scale_matrix = Matrix4::from_nonuniform_scale(node.scale.x, node.scale.y, node.scale.z);
        let rotation_matrix = Matrix4::from_axis_angle(node.rotation_axis, Rad(node.rotation_angle));
        let translation_matrix = Matrix4::from_translation(node.translation2);

        let transform = match node.rotation_keyframe_count > 0 {
            true => translation_matrix * scale_matrix,
            false => translation_matrix * rotation_matrix * scale_matrix,
        };

        let box_transform = parent_matrix * translation_matrix * rotation_matrix * scale_matrix;

        (main, transform, box_transform)
    }

    fn process_node_mesh(
        device: &Device,
        queue: &Queue,
        current_node: &NodeData,
        nodes: &Vec<NodeData>,
        textures: &Vec<Arc<Texture>>,
        parent_matrix: &Matrix4<f32>,
        main_bounding_box: &mut AABB,
        root_node_name: &ModelString<40>,
        reverse_order: bool,
    ) -> Node {
        let (main_matrix, transform_matrix, box_transform_matrix) = Self::calculate_matrices(current_node, parent_matrix);
        let vertices = NativeModelVertex::to_vertices(Self::make_vertices(current_node, &main_matrix, reverse_order));

        let vertex_buffer = Buffer::with_data(
            device,
            queue,
            &current_node.node_name.inner,
            BufferUsages::COPY_DST | BufferUsages::VERTEX,
            &vertices,
        );

        let box_matrix = box_transform_matrix * main_matrix;
        let bounding_box = AABB::from_vertices(
            current_node
                .vertex_positions
                .iter()
                .map(|position| multiply_matrix4_and_vector3(&box_matrix, *position)),
        );
        main_bounding_box.extend(&bounding_box);

        let final_matrix = match current_node.node_name == *root_node_name {
            true => {
                Matrix4::from_translation(-Vector3::new(
                    bounding_box.center().x,
                    bounding_box.max().y,
                    bounding_box.center().z,
                )) * transform_matrix
            }
            false => transform_matrix,
        };

        let node_textures: Vec<Arc<Texture>> = current_node
            .texture_indices
            .iter()
            .map(|index| *index as usize)
            .map(|index| textures[index].clone())
            .collect();

        let child_nodes = nodes
            .iter()
            .filter(|node| node.parent_node_name == current_node.node_name)
            .filter(|node| node.parent_node_name != node.node_name)
            .map(|node| {
                Self::process_node_mesh(
                    device,
                    queue,
                    node,
                    nodes,
                    textures,
                    &box_transform_matrix,
                    main_bounding_box,
                    root_node_name,
                    reverse_order,
                )
            })
            .collect();

        let node_textures = TextureGroup::new(device, &root_node_name.inner, node_textures);

        Node::new(
            final_matrix,
            vertex_buffer,
            node_textures,
            child_nodes,
            current_node.rotation_keyframes.clone(),
        )
    }

    fn load(&mut self, texture_loader: &mut TextureLoader, model_file: &str, reverse_order: bool) -> Result<Arc<Model>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load rsm model from {}", model_file.magenta()));

        let bytes = self
            .game_file_loader
            .get(&format!("data\\model\\{model_file}"))
            .map_err(LoadError::File)?;
        let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

        let model_data = match ModelData::from_bytes(&mut byte_stream) {
            Ok(model_data) => model_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load model: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.get(texture_loader, FALLBACK_MODEL_FILE, reverse_order);
            }
        };

        let textures = model_data
            .texture_names
            .iter()
            .map(|texture_name| texture_loader.get(&texture_name.inner).unwrap())
            .collect();

        let root_node_name = &model_data.root_node_name;

        let root_node = model_data
            .nodes
            .iter()
            .find(|node_data| &node_data.node_name == root_node_name)
            .expect("failed to find main node");

        let mut bounding_box = AABB::uninitialized();
        let root_node = Self::process_node_mesh(
            &self.device,
            &self.queue,
            root_node,
            &model_data.nodes,
            &textures,
            &Matrix4::identity(),
            &mut bounding_box,
            root_node_name,
            reverse_order,
        );
        let model = Arc::new(Model::new(
            root_node,
            bounding_box,
            #[cfg(feature = "debug")]
            model_data,
        ));

        self.cache.insert((model_file.to_string(), reverse_order), model.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(model)
    }

    pub fn get(&mut self, texture_loader: &mut TextureLoader, model_file: &str, reverse_order: bool) -> Result<Arc<Model>, LoadError> {
        match self.cache.get(&(model_file.to_string(), reverse_order)) {
            // kinda dirty
            Some(model) => Ok(model.clone()),
            None => self.load(texture_loader, model_file, reverse_order),
        }
    }
}
