use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Matrix3, Matrix4, Quaternion, Rad, SquareMatrix, Vector2, Vector3};
use derive_new::new;
use procedural::{Named, *};
use vulkano::image::view::ImageView;

use super::version::InternalVersion;
use super::{conversion_result, ConversionError, FromBytesExt, FALLBACK_MODEL_FILE};
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{BufferAllocator, NativeModelVertex};
use crate::loaders::{ByteStream, FromBytes, GameFileLoader, MajorFirst, TextureLoader, Version};
use crate::system::multiply_matrix4_and_vector3;
use crate::world::{BoundingBox, Model, Node};

#[derive(Debug, Named, FromBytes, PrototypeElement)]
pub struct PositionKeyframeData {
    pub frame: u32,
    pub position: Vector3<f32>,
}

#[derive(Clone, Debug, Named, FromBytes, PrototypeElement)]
pub struct RotationKeyframeData {
    pub frame: u32,
    pub quaternions: Quaternion<f32>,
}

#[allow(dead_code)]
#[derive(Debug, Named, FromBytes, PrototypeElement)]
pub struct FaceData {
    pub vertex_position_indices: [u16; 3],
    pub texture_coordinate_indices: [u16; 3],
    pub texture_index: u16,
    pub padding: u16,
    pub two_sided: i32,
    pub smooth_group: i32,
}

#[derive(Debug, Named, FromBytes, PrototypeElement)]
pub struct TextureCoordinateData {
    #[version_equals_or_above(1, 2)]
    pub color: Option<u32>,
    pub coordinates: Vector2<f32>, // possibly wrong if version < 1.2
}

#[derive(Debug, Named, FromBytes, PrototypeElement)]
pub struct NodeData {
    pub node_name: ModelString<40>,
    pub parent_node_name: ModelString<40>, // This is where 2.2 starts failing
    pub texture_count: u32,
    #[repeating(self.texture_count)]
    pub texture_indices: Vec<u32>,
    #[hidden_element]
    pub offset_matrix: Matrix3<f32>,
    pub translation1: Vector3<f32>,
    pub translation2: Vector3<f32>,
    pub rotation_angle: f32,
    pub rotation_axis: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub vertex_position_count: u32,
    #[repeating(self.vertex_position_count)]
    pub vertex_positions: Vec<Vector3<f32>>,
    pub texture_coordinate_count: u32,
    #[repeating(self.texture_coordinate_count)]
    pub texture_coordinates: Vec<TextureCoordinateData>,
    pub face_count: u32,
    #[repeating(self.face_count)]
    pub faces: Vec<FaceData>,
    #[version_equals_or_above(2, 5)] // unsure what vesion this is supposed to be (must be > 1.5)
    pub position_keyframe_count: Option<u32>,
    #[repeating(self.position_keyframe_count.unwrap_or_default())]
    pub position_keyframes: Vec<PositionKeyframeData>,
    pub rotation_keyframe_count: u32,
    #[repeating(self.rotation_keyframe_count)]
    pub rotation_keyframes: Vec<RotationKeyframeData>,
}

#[derive(Clone, Debug, PartialEq, Eq, Named)]
pub struct ModelString<const LENGTH: usize> {
    pub inner: String,
}

impl<const LENGTH: usize> FromBytes for ModelString<LENGTH> {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> Result<Self, Box<ConversionError>> {
        let inner = if byte_stream
            .get_metadata::<Self, Option<InternalVersion>>()?
            .ok_or(ConversionError::from_message("version not set"))?
            .equals_or_above(2, 2)
        {
            let length = conversion_result::<Self, _>(u32::from_bytes(byte_stream))? as usize;
            let mut inner = conversion_result::<Self, _>(String::from_n_bytes(byte_stream, length))?;
            // need to remove the last character for some reason
            inner.pop();
            inner
        } else {
            conversion_result::<Self, _>(String::from_n_bytes(byte_stream, LENGTH))?
        };

        Ok(Self { inner })
    }
}

impl<const LENGTH: usize> crate::interface::PrototypeElement for ModelString<LENGTH> {
    fn to_element(&self, display: String) -> crate::interface::ElementCell {
        self.inner.to_element(display)
    }
}

#[derive(Debug, Named, FromBytes, PrototypeElement)]
pub struct ModelData {
    #[version]
    pub version: Version<MajorFirst>,
    pub animation_length: u32,
    pub shade_type: u32,
    #[version_equals_or_above(1, 4)]
    pub alpha: Option<u8>,
    #[version_smaller(2, 2)]
    pub reserved0: Option<[u8; 16]>,
    #[version_equals_or_above(2, 2)]
    pub reserved1: Option<[u8; 4]>,
    pub texture_count: u32,
    #[repeating(self.texture_count)]
    pub texture_names: Vec<ModelString<40>>,
    #[version_equals_or_above(2, 2)]
    pub skip: Option<u32>,
    pub root_node_name: ModelString<40>,
    pub node_count: u32,
    #[repeating(self.node_count)]
    pub nodes: Vec<NodeData>,
}

#[derive(new)]
pub struct ModelLoader {
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
        buffer_allocator: &mut BufferAllocator,
        current_node: &NodeData,
        nodes: &Vec<NodeData>,
        textures: &Vec<Arc<ImageView>>,
        parent_matrix: &Matrix4<f32>,
        main_bounding_box: &mut BoundingBox,
        root_node_name: &ModelString<40>,
        reverse_order: bool,
    ) -> Node {
        let (main_matrix, transform_matrix, box_transform_matrix) = Self::calculate_matrices(current_node, parent_matrix);
        let vertices = NativeModelVertex::to_vertices(Self::make_vertices(current_node, &main_matrix, reverse_order));

        let vertex_buffer = buffer_allocator.allocate_vertex_buffer(vertices);

        let box_matrix = box_transform_matrix * main_matrix;
        let bounding_box = BoundingBox::new(
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
                    bounding_box.biggest.y,
                    bounding_box.center().z,
                )) * transform_matrix
            }
            false => transform_matrix,
        };

        let node_textures = current_node
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
                    buffer_allocator,
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

        Node::new(
            final_matrix,
            vertex_buffer,
            node_textures,
            child_nodes,
            current_node.rotation_keyframes.clone(),
        )
    }

    fn load(
        &mut self,
        buffer_allocator: &mut BufferAllocator,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        model_file: &str,
        reverse_order: bool,
    ) -> Result<Arc<Model>, String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load rsm model from {MAGENTA}{model_file}{NONE}"));

        let bytes = game_file_loader.get(&format!("data\\model\\{model_file}"))?;
        let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

        if <[u8; 4]>::from_bytes(&mut byte_stream).unwrap() != [b'G', b'R', b'S', b'M'] {
            return Err(format!("failed to read magic number from {model_file}"));
        }

        let model_data = match ModelData::from_bytes(&mut byte_stream) {
            Ok(model_data) => model_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load model: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.get(
                    buffer_allocator,
                    game_file_loader,
                    texture_loader,
                    FALLBACK_MODEL_FILE,
                    reverse_order,
                );
            }
        };

        let textures = model_data
            .texture_names
            .iter()
            .map(|texture_name| texture_loader.get(&texture_name.inner, game_file_loader).unwrap())
            .collect();

        let root_node_name = &model_data.root_node_name;

        let root_node = model_data
            .nodes
            .iter()
            .find(|node_data| &node_data.node_name == root_node_name)
            .expect("failed to find main node");

        let mut bounding_box = BoundingBox::uninitialized();
        let root_node = Self::process_node_mesh(
            buffer_allocator,
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

    pub fn get(
        &mut self,
        buffer_allocator: &mut BufferAllocator,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        model_file: &str,
        reverse_order: bool,
    ) -> Result<Arc<Model>, String> {
        match self.cache.get(&(model_file.to_string(), reverse_order)) {
            // kinda dirty
            Some(model) => Ok(model.clone()),
            None => self.load(buffer_allocator, game_file_loader, texture_loader, model_file, reverse_order),
        }
    }
}
