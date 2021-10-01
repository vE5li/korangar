mod partial;
mod model;

use std::sync::Arc;
use std::collections::HashMap;
use std::fs::read;

use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::device::Device;

#[cfg(feature = "debug")]
use debug::*;

use cgmath::{ Vector2, Vector3, InnerSpace };

use managers::TextureManager;

use super::ByteStream;
use self::partial::*;
use self::model::{ Model, Node, ShadingType };

pub struct ModelManager {
    cache: HashMap<String, Arc<Model>>,
    device: Arc<Device>,
}

impl ModelManager {

    pub fn new(device: Arc<Device>) -> Self {
        return Self {
            cache: HashMap::new(),
            device: device,
        }
    }

    fn load(&mut self, texture_manager: &mut TextureManager, path: String) -> Arc<Model> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load rsm model from {}{}{}", magenta(), path, none()));

        let bytes = read(path.clone()).expect("u r stupid");
        let mut byte_stream = ByteStream::new(bytes.iter());

        let magic = byte_stream.string(4);
        assert!(&magic == "GRSM", "failed to read magic number");

        let version = byte_stream.version();
        let animation_length = byte_stream.integer(4);
        let shading_type = ShadingType::from(byte_stream.integer(4) as usize);

        let alpha = match version.equals_or_above(1, 4) {
            true => byte_stream.integer(1),
            false => 255,
        };

        byte_stream.skip(16);

        let texture_count = byte_stream.integer(4);
        let mut textures = Vec::new();

        for _index in 0..texture_count as usize {
            let texture_name = byte_stream.string(40);
            let texture_name_unix = texture_name.replace("\\", "/");
            let (texture, mut future) = texture_manager.get(texture_name_unix);

            // todo return gpu future instead
            future.cleanup_finished();
            textures.push(texture);
        }

        let main_node_name = byte_stream.string(40);
        let node_count = byte_stream.integer(4);

        #[cfg(feature = "debug_model")]
        {
            print_debug!("version {}{}{}", magenta(), version, none());
            print_debug!("animation length {}{}{}", magenta(), animation_length, none());
            print_debug!("shading type {}{}{}", magenta(), shading_type, none());
            print_debug!("alpha {}{}{}", magenta(), alpha, none());
            print_debug!("texture count {}{}{}", magenta(), texture_count, none());
            print_debug!("texture count {}{}{}", magenta(), texture_count, none());
            print_debug!("main node name {}{}{}", magenta(), main_node_name, none());
            print_debug!("node count {}{}{}", magenta(), node_count, none());
        }

        let mut nodes = Vec::new();

        for _index in 0..node_count as usize {

            let node_name = byte_stream.string(40);
            let parent_name = byte_stream.string(40);

            #[cfg(feature = "debug_model")]
            let timer = Timer::new_dynamic(format!("parse node {}{}{}", magenta(), node_name, none()));

            let texture_count = byte_stream.integer(4);

            let mut texture_indices = Vec::new();
            let mut node_textures = Vec::new();

            for _index in 0..texture_count {
                let texture_index = byte_stream.integer(4) as usize;
                texture_indices.push(texture_index);
                node_textures.push(textures[texture_index].clone());
            }

            let _offset_matrix = byte_stream.slice(36); // matrix 3x3
            let offset_translation = byte_stream.vector3();
            let translation = byte_stream.vector3();
            let rotation_angle = byte_stream.float32();
            let rotation_axis = byte_stream.vector3();
            let scale = byte_stream.vector3();

            let vertex_count = byte_stream.integer(4) as usize;

            let mut vertex_positions = Vec::new();
            let mut common_normals = Vec::new();

            for _index in 0..vertex_count {
                let vertex_position = byte_stream.vector3();
                let dirty = Vector3::new(vertex_position.x, vertex_position.z, vertex_position.y);
                vertex_positions.push(dirty);
                common_normals.push(Vec::new());
            }

            let texture_coordinate_count = byte_stream.integer(4);

            let mut texture_coordinates = Vec::new();

            for _index in 0..texture_coordinate_count {
                if version.equals_or_above(1, 2) {

                    let _color = byte_stream.integer(4);
                    let u = byte_stream.float32();
                    let v = byte_stream.float32();

                    texture_coordinates.push(Vector2::new(u, v)); // color
                } else {

                    let u = byte_stream.float32();
                    let v = byte_stream.float32();

                    texture_coordinates.push(Vector2::new(u, v));
                }
            }

            let face_count = byte_stream.integer(4);

            let mut vertices = Vec::new();
            let mut partial_vertices = Vec::new();

            for _index in 0..face_count {

                let first_vertex_position_index = byte_stream.integer(2);
                let second_vertex_position_index = byte_stream.integer(2);
                let third_vertex_position_index = byte_stream.integer(2);

                let first_texture_coordinate_index = byte_stream.integer(2);
                let second_texture_coordinate_index = byte_stream.integer(2);
                let third_texture_coordinate_index = byte_stream.integer(2);

                let texture_index = byte_stream.integer(2) as i32;
                byte_stream.skip(2);
                let _double_sided = byte_stream.integer(4);

                let _smooth_group = match version.equals_or_above(1, 2) {
                    true => byte_stream.integer(4),
                    false => 0,
                };

                let offset = partial_vertices.len();
                common_normals[first_vertex_position_index as usize].push(offset);
                common_normals[second_vertex_position_index as usize].push(offset + 1);
                common_normals[third_vertex_position_index as usize].push(offset + 2);

                let first_vertex_position = vertex_positions[first_vertex_position_index as usize];
                let second_vertex_position = vertex_positions[second_vertex_position_index as usize];
                let third_vertex_position = vertex_positions[third_vertex_position_index as usize];

                let first_texture_coordinate = texture_coordinates[first_texture_coordinate_index as usize];
                let second_texture_coordinate = texture_coordinates[second_texture_coordinate_index as usize];
                let third_texture_coordinate = texture_coordinates[third_texture_coordinate_index as usize];

                let normal = calculate_normal(first_vertex_position, second_vertex_position, third_vertex_position);

                partial_vertices.push(PartialVertex::new(first_vertex_position, normal, first_texture_coordinate, texture_index));
                partial_vertices.push(PartialVertex::new(second_vertex_position, normal, second_texture_coordinate, texture_index));
                partial_vertices.push(PartialVertex::new(third_vertex_position, normal, third_texture_coordinate, texture_index));
            }

            if version.equals_or_above(1, 5) {
                panic!("animation key frames not implemented");
            }

            let rotation_key_frame_count = byte_stream.integer(4);

            for _index in 0..rotation_key_frame_count {
                let _time = byte_stream.integer(4);
                let _orientation = byte_stream.slice(16); // quat
                // push
            }

            for normal_group in common_normals {
                if normal_group.len() < 2 {
                    continue;
                }

                let new_normal = normal_group.iter()
                    .map(|index| partial_vertices[*index].normal)
                    .fold(Vector3::new(0.0, 0.0, 0.0), |output, normal| output + normal);

                normal_group.iter().for_each(|index| partial_vertices[*index].normal = new_normal);
            }

            while !partial_vertices.is_empty() {

                let mut first_partial = partial_vertices.remove(0);
                let mut second_partial = partial_vertices.remove(0);
                let mut third_partial = partial_vertices.remove(0);

                first_partial.normal = first_partial.normal.normalize();
                second_partial.normal = second_partial.normal.normalize();
                third_partial.normal = third_partial.normal.normalize();

                vertices.push(first_partial.to_vertex());
                vertices.push(second_partial.to_vertex());
                vertices.push(third_partial.to_vertex());
            }

            let parent_name = match parent_name.is_empty() {
                true => None,
                false => Some(parent_name),
            };

            let adjusted_translation = Vector3::new(translation.x, -translation.y, translation.z);
            let vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();
            let node = Node::new(node_name.clone(), parent_name.clone(), node_textures, adjusted_translation, vertex_count, vertex_buffer);

            nodes.push(node);

            #[cfg(feature = "debug_model")]
            {
                parent_name.map(|name| print_debug!("parent name {}{}{}", magenta(), name, none()));

                let formatted_list = texture_indices.iter().map(|index| index.to_string()).collect::<Vec<String>>().join(", ");
                print_debug!("texture count {}{}{}", magenta(), texture_count, none());
                print_debug!("texture indices {}{}{}", magenta(), formatted_list, none());

                print_debug!("offset tranlation {}{:?}{}", magenta(), offset_translation, none());
                print_debug!("translation {}{:?}{}", magenta(), translation, none());
                print_debug!("rotation angle {}{}{}", magenta(), rotation_angle, none());
                print_debug!("rotation axis {}{:?}{}", magenta(), rotation_axis, none());
                print_debug!("scale {}{:?}{}", magenta(), scale, none());

                print_debug!("vertex count {}{}{}", magenta(), vertex_count, none());
                print_debug!("texture coordinate count {}{}{}", magenta(), texture_coordinate_count, none());
                print_debug!("face count {}{}{}", magenta(), face_count, none());
                print_debug!("rotation key frame count {}{}{}", magenta(), rotation_key_frame_count, none());

                timer.stop();
            }
        }

        for node in nodes.clone().iter() { // fix ordering issue
            if let Some(parent_name) = &node.parent_name {
                let parent_node = nodes.iter_mut().find(|node| node.name == *parent_name).expect("failed to find parent node");
                parent_node.child_nodes.push(node.clone());
            }
        }

        let root_node = nodes.iter().find(|node| node.name == *main_node_name).expect("failed to find root node").clone(); // fix cloning issue
        let model = Arc::new(Model::new(root_node));

        self.cache.insert(path, model.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        return model;
    }

    pub fn get(&mut self, texture_manager: &mut TextureManager, path: String) -> Arc<Model> {
        match self.cache.get(&path) {
            Some(model) => return model.clone(),
            None => return self.load(texture_manager, path),
        }
    }
}
