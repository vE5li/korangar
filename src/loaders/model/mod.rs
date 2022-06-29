use derive_new::new;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::collections::HashMap;
use types::maths::*;
use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::device::Device;
use vulkano::sync::GpuFuture;

#[cfg(feature = "debug")]
use debug::*;
use types::ByteStream;
use types::map::model::{ Model, Node, BoundingBox, ShadingType };
use graphics::{ Transform, NativeModelVertex };
use loaders::{ TextureLoader, GameFileLoader };

#[derive(new)]
pub struct ModelLoader {
    game_file_loader: Rc<RefCell<GameFileLoader>>,
    device: Arc<Device>,
    #[new(default)]
    cache: HashMap<String, Arc<Model>>,
}

impl ModelLoader {

    fn calculate_bounding_box(nodes: &Vec<Node>) -> BoundingBox {

        let mut smallest: Vector3<f32> = vector3!(999999.0);
        let mut biggest: Vector3<f32> = vector3!(-999999.0);

        for node in nodes {
            smallest.x = smallest.x.min(node.bounding_box.smallest.x);
            smallest.y = smallest.y.min(node.bounding_box.smallest.y);
            smallest.z = smallest.z.min(node.bounding_box.smallest.z);

            biggest.x = biggest.x.max(node.bounding_box.biggest.x);
            biggest.y = biggest.y.max(node.bounding_box.biggest.y);
            biggest.z = biggest.z.max(node.bounding_box.biggest.z);
        }

        let offset = (biggest + smallest).map(|component| component / 2.0);
        let range = (biggest - smallest).map(|component| component / 2.0);

        BoundingBox::new(smallest, biggest, offset, range)
    }

    fn calculate_node_bounding_box(vertices: &Vec<NativeModelVertex>, offset_matrix: Matrix3<f32>, offset_translation: Vector3<f32>, is_only: bool) -> BoundingBox {

        let mut smallest: Vector3<f32> = vector3!(999999.0);
        let mut biggest: Vector3<f32> = vector3!(-999999.0);

        for vertex in vertices {
            let mut vv = offset_matrix * vertex.position;

            if !is_only {
                vv += offset_translation;
            }

            smallest.x = smallest.x.min(vv.x);
            smallest.y = smallest.y.min(vv.y);
            smallest.z = smallest.z.min(vv.z);

            biggest.x = biggest.x.max(vv.x);
            biggest.y = biggest.y.max(vv.y);
            biggest.z = biggest.z.max(vv.z);
        }

        let offset = (biggest + smallest).map(|component| component / 2.0);
        let range = (biggest - smallest).map(|component| component / 2.0);

        BoundingBox::new(smallest, biggest, offset, range)
    }

    fn load(&mut self, texture_loader: &mut TextureLoader, model_file: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Arc<Model>, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load rsm model from {}{}{}", MAGENTA, model_file, NONE));

        let bytes = self.game_file_loader.borrow_mut().get(&format!("data\\model\\{}", model_file))?;
        let mut byte_stream = ByteStream::new(&bytes);

        let magic = byte_stream.string(4);
        
        if &magic != "GRSM" {
            return Err(format!("failed to read magic number from {}{}{}", MAGENTA, model_file, NONE));
        }

        let version = byte_stream.version();
        let _animation_length = byte_stream.integer32();
        let _shading_type = ShadingType::from(byte_stream.integer32() as usize);

        let _alpha = match version.equals_or_above(1, 4) {
            true => byte_stream.byte(),
            false => 255,
        };

        byte_stream.skip(16);

        let texture_count = byte_stream.integer32();
        let mut textures = Vec::new();

        for _index in 0..texture_count as usize {
            let texture_name = byte_stream.string(40);
            //let texture_name_unix = texture_name.replace("\\", "/");
            let texture = texture_loader.get(&texture_name, texture_future)?;
            textures.push(texture);
        }

        let main_node_name = byte_stream.string(40);
        let node_count = byte_stream.integer32();

        #[cfg(feature = "debug_model")]
        {
            print_debug!("version {}{}{}", MAGENTA, version, NONE);
            print_debug!("animation length {}{}{}", MAGENTA, _animation_length, NONE);
            print_debug!("shading type {}{}{}", MAGENTA, _shading_type, NONE);
            print_debug!("alpha {}{}{}", MAGENTA, _alpha, NONE);
            print_debug!("texture count {}{}{}", MAGENTA, texture_count, NONE);
            print_debug!("main node name {}{}{}", MAGENTA, main_node_name, NONE);
            print_debug!("node count {}{}{}", MAGENTA, node_count, NONE);
        }

        let mut nodes = Vec::new();

        for _index in 0..node_count as usize {

            let node_name = byte_stream.string(40);
            let parent_name = byte_stream.string(40);

            #[cfg(feature = "debug_model")]
            let timer = Timer::new_dynamic(format!("parse node {}{}{}", MAGENTA, node_name, NONE));

            let texture_count = byte_stream.integer32();

            let mut texture_indices = Vec::new();
            let mut node_textures = Vec::new();

            for _index in 0..texture_count {
                let texture_index = byte_stream.integer32() as usize;
                texture_indices.push(texture_index);
                node_textures.push(textures[texture_index].clone());
            }

            let offset_matrix = byte_stream.matrix3();
            let offset_translation = byte_stream.vector3();
            let position = byte_stream.vector3();
            let rotation_angle = byte_stream.float32();
            let rotation_axis = byte_stream.vector3();
            let scale = byte_stream.vector3();

            let vertex_count = byte_stream.integer32() as usize;

            let mut vertex_positions = Vec::new();
            let mut common_normals = Vec::new();

            for _index in 0..vertex_count {
                let vertex_position = byte_stream.vector3();
                let dirty = Vector3::new(vertex_position.x, vertex_position.y, -vertex_position.z);

                vertex_positions.push(dirty);
                common_normals.push(Vec::new());
            }

            let texture_coordinate_count = byte_stream.integer32();

            let mut texture_coordinates = Vec::new();

            for _index in 0..texture_coordinate_count {
                if version.equals_or_above(1, 2) {

                    let _color = byte_stream.integer32();
                    let u = byte_stream.float32();
                    let v = byte_stream.float32();
                    texture_coordinates.push(Vector2::new(u, v)); // color
                } else {

                    let u = byte_stream.float32();
                    let v = byte_stream.float32();
                    texture_coordinates.push(Vector2::new(u, v));
                }
            }

            let face_count = byte_stream.integer32();

            //let mut vertices = Vec::new();
            let mut native_vertices = Vec::new();

            for _index in 0..face_count {

                let first_vertex_position_index = byte_stream.integer16();
                let second_vertex_position_index = byte_stream.integer16();
                let third_vertex_position_index = byte_stream.integer16();

                let first_texture_coordinate_index = byte_stream.integer16();
                let second_texture_coordinate_index = byte_stream.integer16();
                let third_texture_coordinate_index = byte_stream.integer16();

                let texture_index = byte_stream.integer16() as i32;
                byte_stream.skip(2);
                let _double_sided = byte_stream.integer32();

                let _smooth_group = match version.equals_or_above(1, 2) {
                    true => byte_stream.integer32(),
                    false => 0,
                };

                let offset = native_vertices.len();
                common_normals[first_vertex_position_index as usize].push(offset);
                common_normals[second_vertex_position_index as usize].push(offset + 1);
                common_normals[third_vertex_position_index as usize].push(offset + 2);

                let first_vertex_position = vertex_positions[first_vertex_position_index as usize];
                let second_vertex_position = vertex_positions[second_vertex_position_index as usize];
                let third_vertex_position = vertex_positions[third_vertex_position_index as usize];

                let first_texture_coordinate = texture_coordinates[first_texture_coordinate_index as usize];
                let second_texture_coordinate = texture_coordinates[second_texture_coordinate_index as usize];
                let third_texture_coordinate = texture_coordinates[third_texture_coordinate_index as usize];

                let normal = NativeModelVertex::calculate_normal(first_vertex_position, second_vertex_position, third_vertex_position);

                native_vertices.push(NativeModelVertex::new(first_vertex_position, normal, first_texture_coordinate, texture_index));
                native_vertices.push(NativeModelVertex::new(second_vertex_position, normal, second_texture_coordinate, texture_index));
                native_vertices.push(NativeModelVertex::new(third_vertex_position, normal, third_texture_coordinate, texture_index));
            }

            if version.equals_or_above(1, 5) {
                panic!("animation key frames not implemented");
            }

            let rotation_key_frame_count = byte_stream.integer32();

            for _index in 0..rotation_key_frame_count {
                let _time = byte_stream.integer32();
                let _orientation = byte_stream.slice(16); // quat
                // push
            }

            //for normal_group in common_normals {
            //    if normal_group.len() < 2 {
            //        continue;
            //    }

            //    let new_normal = normal_group.iter()
            //        .map(|index| native_vertices[*index].normal)
            //        .fold(Vector3::new(0.0, 0.0, 0.0), |output, normal| output + normal);

            //    normal_group.iter().for_each(|index| native_vertices[*index].normal = new_normal);
            //}

            let parent_name = match parent_name.is_empty() {
                true => None,
                false => Some(parent_name),
            };

            let bounding_box = Self::calculate_node_bounding_box(&native_vertices, offset_matrix, offset_translation, true);

            let rotation = vector3!(Rad(0.0)); // get from axis and angle

            //let scale = scale.map(|c| c.abs());

            let vertices = NativeModelVertex::to_vertices(native_vertices);
            let vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();
            let transform = Transform::offset(-offset_translation) + Transform::offset_matrix(offset_matrix.into());

            nodes.push(Node::new(node_name.clone(), parent_name.clone(), node_textures, transform, vertex_buffer, bounding_box, offset_matrix, offset_translation, position, rotation, scale));

            #[cfg(feature = "debug_model")]
            {
                parent_name.map(|name| print_debug!("parent name {}{}{}", MAGENTA, name, NONE));

                let formatted_list = texture_indices.iter().map(|index| index.to_string()).collect::<Vec<String>>().join(", ");
                print_debug!("texture count {}{}{}", MAGENTA, texture_count, NONE);
                print_debug!("texture indices {}{}{}", MAGENTA, formatted_list, NONE);

                print_debug!("offset matrix {}{:?}{}", MAGENTA, offset_matrix, NONE);
                print_debug!("offset tranlation {}{:?}{}", MAGENTA, offset_translation, NONE);
                print_debug!("position {}{:?}{}", MAGENTA, position, NONE);
                print_debug!("rotation angle {}{}{}", MAGENTA, rotation_angle, NONE);
                print_debug!("rotation axis {}{:?}{}", MAGENTA, rotation_axis, NONE);
                print_debug!("scale {}{:?}{}", MAGENTA, scale, NONE);

                print_debug!("ModelVertex count {}{}{}", MAGENTA, vertex_count, NONE);
                print_debug!("texture coordinate count {}{}{}", MAGENTA, texture_coordinate_count, NONE);
                print_debug!("face count {}{}{}", MAGENTA, face_count, NONE);
                print_debug!("rotation key frame count {}{}{}", MAGENTA, rotation_key_frame_count, NONE);

                timer.stop();
            }
        }

        // always 8 x 0x0 ?
        let _unknown = byte_stream.slice(8);

        #[cfg(feature = "debug")]
        byte_stream.assert_empty(&model_file);

        let bounding_box = Self::calculate_bounding_box(&nodes);

        for node in nodes.clone().iter() { // fix ordering issue
            if let Some(parent_name) = &node.parent_name {
                let parent_node = nodes.iter_mut().find(|node| node.name == *parent_name).expect("failed to find parent node");
                parent_node.child_nodes.push(node.clone());
            }
        }

        let root_node = nodes.iter().find(|node| node.name == *main_node_name).expect("failed to find root node").clone(); // fix cloning issue
        let model = Arc::new(Model::new(root_node, bounding_box));

        self.cache.insert(model_file.to_string(), model.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(model)
    }

    pub fn get(&mut self, texture_loader: &mut TextureLoader, model_file: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Arc<Model>, String> {
        match self.cache.get(model_file) {
            Some(model) => Ok(model.clone()),
            None => self.load(texture_loader, model_file, texture_future),
        }
    }
}
