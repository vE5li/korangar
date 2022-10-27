use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Matrix3, Matrix4, Quaternion, Rad, SquareMatrix, Vector2, Vector3};
use derive_new::new;
use procedural::*;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::device::Device;
use vulkano::sync::GpuFuture;

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{NativeModelVertex, Texture};
use crate::loaders::{ByteConvertable, ByteStream, GameFileLoader, TextureLoader, Version};
use crate::system::multiply_matrix4_and_vector3;
use crate::world::{BoundingBox, Model, Node};

#[derive(Debug, ByteConvertable, PrototypeElement)]
pub struct PositionKeyframeData {
    pub frame: u32,
    pub position: Vector3<f32>,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub struct RotationKeyframeData {
    pub frame: u32,
    pub quaternions: Quaternion<f32>,
}

#[allow(dead_code)]
#[derive(Debug, ByteConvertable, PrototypeElement)]
pub struct FaceData {
    pub vertex_position_indices: [u16; 3],
    pub texture_coordinate_indices: [u16; 3],
    pub texture_index: u16,
    pub padding: u16,
    pub two_sided: i32,
    pub smooth_group: i32,
}

#[derive(Debug, ByteConvertable, PrototypeElement)]
pub struct TextureCoordinateData {
    #[version_equals_or_above(1, 2)]
    pub color: Option<u32>,
    pub coordinates: Vector2<f32>, // possibly wrong if version < 1.2
}

#[derive(Debug, ByteConvertable, PrototypeElement)]
pub struct NodeData {
    #[length_hint(40)]
    pub node_name: ModelString,
    #[length_hint(40)]
    pub parent_node_name: ModelString, // This is where 2.2 starts failing
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

#[derive(Clone, Debug, PartialEq, Eq, PrototypeElement)]
pub struct ModelString {
    pub inner: String,
}

impl ByteConvertable for ModelString {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {

        let inner = if byte_stream.get_version().equals_or_above(2, 2) {

            let length = u32::from_bytes(byte_stream, None) as usize;
            let mut inner = String::from_bytes(byte_stream, Some(length));
            // need to remove the last character for some reason
            inner.pop();
            inner
        } else {
            String::from_bytes(byte_stream, length_hint)
        };

        Self { inner }
    }
}

#[derive(Debug, ByteConvertable, PrototypeElement)]
pub struct ModelData {
    //#[version]
    //pub version: Version,
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
    #[length_hint(40)]
    pub texture_names: Vec<ModelString>,
    #[version_equals_or_above(2, 2)]
    pub skip: Option<u32>,
    #[length_hint(40)]
    pub root_node_name: ModelString,
    pub node_count: u32,
    #[repeating(self.node_count)]
    pub nodes: Vec<NodeData>,
}

#[derive(new)]
pub struct ModelLoader {
    device: Arc<Device>,
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
        device: Arc<Device>,
        current_node: &NodeData,
        nodes: &Vec<NodeData>,
        textures: &Vec<Texture>,
        parent_matrix: &Matrix4<f32>,
        main_bounding_box: &mut BoundingBox,
        root_node_name: &ModelString,
        reverse_order: bool,
    ) -> Node {

        let (main_matrix, transform_matrix, box_transform_matrix) = Self::calculate_matrices(current_node, parent_matrix);
        let vertices = NativeModelVertex::to_vertices(Self::make_vertices(current_node, &main_matrix, reverse_order));
        let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();

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
                    device.clone(),
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
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        texture_future: &mut Box<dyn GpuFuture + 'static>,
        model_file: &str,
        reverse_order: bool,
    ) -> Result<Arc<Model>, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load rsm model from {}{}{}", MAGENTA, model_file, NONE));

        let bytes = game_file_loader.get(&format!("data\\model\\{}", model_file))?;
        let mut byte_stream = ByteStream::new(&bytes);

        let magic = byte_stream.string(4);

        if &magic != "GRSM" {
            return Err(format!("failed to read magic number from {}", model_file));
        }

        // make this prettier
        let major = byte_stream.next();
        let minor = byte_stream.next();
        byte_stream.set_version(Version::new(major, minor));

        let model_data = ModelData::from_bytes(&mut byte_stream, None);

        let textures = model_data
            .texture_names
            .iter()
            .map(|texture_name| texture_loader.get(&texture_name.inner, game_file_loader, texture_future).unwrap())
            .collect();

        let root_node_name = &model_data.root_node_name;

        let root_node = model_data
            .nodes
            .iter()
            .find(|node_data| &node_data.node_name == root_node_name)
            .expect("failed to find main node");

        let mut bounding_box = BoundingBox::uninitialized();
        let root_node = Self::process_node_mesh(
            self.device.clone(),
            root_node,
            &model_data.nodes,
            &textures,
            &Matrix4::identity(),
            &mut bounding_box,
            &root_node_name,
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
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        texture_future: &mut Box<dyn GpuFuture + 'static>,
        model_file: &str,
        reverse_order: bool,
    ) -> Result<Arc<Model>, String> {
        match self.cache.get(&(model_file.to_string(), reverse_order)) {
            // kinda dirty
            Some(model) => Ok(model.clone()),
            None => self.load(game_file_loader, texture_loader, texture_future, model_file, reverse_order),
        }
    }
}
