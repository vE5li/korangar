mod stream;
mod version;
mod partial;

use std::collections::HashMap;
use std::fs::read_to_string;
use std::fs::read;
use std::slice::Iter;
use std::sync::Arc;

use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::device::Device;

#[cfg(feature = "debug")]
use debug::*;

use cgmath::{ Vector2, Vector3, InnerSpace };
use graphics::{ VertexBuffer, Vertex };

use self::version::Version;
use self::stream::ByteStream;
//use self::partial::PartialVertex;
use self::partial::*;

enum ShadingType {
    None,
    FlatShading,
    SmoothShading,
    Black,
}

impl ShadingType {

    pub fn from(raw: usize) -> Self {
        match raw {
            0 => return ShadingType::None,
            1 => return ShadingType::FlatShading,
            2 => return ShadingType::SmoothShading,
            3 => return ShadingType::Black,
            invalid => panic!("invalid shading type"), // return result ?
        }
    }
}

pub struct Node {
    name: String,
    parent_node: Option<Arc<Node>>,
    vertex_buffer: VertexBuffer,
    //textures: Vec<Texture>,
}

pub struct Model {
    nodes: Vec<Arc<Node>>,
}

pub struct ModelManager {
    cache: HashMap<String, VertexBuffer>,
    device: Arc<Device>,
}

impl ModelManager {

    pub fn new(device: Arc<Device>) -> Self {
        return Self {
            cache: HashMap::new(),
            device: device,
        }
    }

    fn load(&mut self, path: String) -> VertexBuffer {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load rsm model from {}{}{}", magenta(), path, none()));

        let bytes = read(path.clone()).expect("u r stupid");
        let mut byte_stream = ByteStream::new(bytes.iter());

        let magic = byte_stream.string(4);
        assert!(&magic == "GRSM", "failed to read magic number");

        let version = byte_stream.version();

        println!("\nversion {:?}", version);

        let animation_length = byte_stream.integer(4);
        let shade_type = byte_stream.integer(4);

        println!("\nanimation length {}", animation_length);
        println!("shade type {}", shade_type);

        if version.equals_or_above(1, 4) {

            let alpha = byte_stream.integer(1);

            println!("\nalpha {}", alpha);
        }

        byte_stream.skip(16);

        let texture_count = byte_stream.integer(4);

        println!("\ntexture count {}", texture_count);

        let mut texture_names = Vec::new();

        for _index in 0..texture_count as usize {
            let texture_name = byte_stream.string(40);
            texture_names.push(texture_name);
        }

        println!("\ntexture names {:?}", texture_names);

        let main_node_name = byte_stream.string(40);
        let node_count = byte_stream.integer(4);

        println!("\nmain node name {:?}", main_node_name);
        println!("node count {}", node_count);

        //let mut nodes = Vec::new();

        // TEMP: push all vertices into one buffer
        let mut vertices = Vec::new();

        for _index in 0..node_count as usize {

            let node_name = byte_stream.string(40);
            let parent_name = byte_stream.string(40);

            println!("\nnode name {:?}", node_name);
            println!("parent name {:?}", parent_name);

            let texture_count = byte_stream.integer(4);

            println!("\ntexture count {}", texture_count);

            let mut texture_indices = Vec::new();

            for _index in 0..texture_count {
                let texture_index = byte_stream.integer(4);
                texture_indices.push(texture_index);
            }

            println!("\ntexture indices {:?}", texture_indices);

            let offset_matrix = byte_stream.slice(36); // matrix 4x4
            let offset_translation = byte_stream.vector3();

            println!("\noffset matrix {:?}", offset_matrix);
            println!("offset tranlation {:?}", offset_translation);

            let translation = byte_stream.vector3();
            let rotation_angle = byte_stream.float32();
            let rotation_axis = byte_stream.vector3();
            let scale = byte_stream.vector3();

            println!("\ntranslation {:?}", translation);
            println!("rotation angle {}", rotation_angle);
            println!("rotation axis {:?}", rotation_axis);
            println!("scale {:?}", scale);

            let vertex_count = byte_stream.integer(4);

            println!("\nvertex count {}", vertex_count);

            let mut vertex_positions = Vec::new();
            let mut common_normals = Vec::new();

            for _index in 0..vertex_count {
                let vertex_position = byte_stream.vector3();
                let dirty = vertex_position + Vector3::new(translation.x, translation.z, -translation.y - 6.0);
                let dirty2 = Vector3::new(dirty.x, dirty.z, dirty.y);
                vertex_positions.push(dirty2);
                common_normals.push(Vec::new());
            }

            let texture_coordinate_count = byte_stream.integer(4);

            println!("\ntexture coordinate count {}", texture_coordinate_count);

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

            println!("\nface count {}", face_count);

            //let mut vertices = Vec::new();
            let mut partial_vertices = Vec::new();

            for _index in 0..face_count {

                let first_vertex_position_index = byte_stream.integer(2);
                let second_vertex_position_index = byte_stream.integer(2);
                let third_vertex_position_index = byte_stream.integer(2);

                let first_texture_coordinate_index = byte_stream.integer(2);
                let second_texture_coordinate_index = byte_stream.integer(2);
                let third_texture_coordinate_index = byte_stream.integer(2);

                let texture_index = byte_stream.integer(2);
                byte_stream.skip(2);
                let double_sided = byte_stream.integer(4); // needed?

                let smooth_group = match version.equals_or_above(1, 2) {
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

                partial_vertices.push(PartialVertex::new(first_vertex_position, normal, first_texture_coordinate));
                partial_vertices.push(PartialVertex::new(second_vertex_position, normal, second_texture_coordinate));
                partial_vertices.push(PartialVertex::new(third_vertex_position, normal, third_texture_coordinate));
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

            if version.equals_or_above(1, 5) {
                panic!("animation key frames not implemented");
            }

            let rotation_key_frame_count = byte_stream.integer(4);

            println!("\nrotation key frame count {}", rotation_key_frame_count);

            for _index in 0..rotation_key_frame_count {
                let time = byte_stream.integer(4);
                let orientation = byte_stream.slice(16); // quat
                // push
            }
        }

        let vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();
        self.cache.insert(path, vertex_buffer.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        return vertex_buffer;
    }

    pub fn get(&mut self, path: String) -> VertexBuffer {
        match self.cache.get(&path) {
            Some(model) => return model.clone(),
            None => return self.load(path),
        }
    }
}
