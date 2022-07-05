use derive_new::new;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::collections::HashMap;
use crate::types::maths::*;
use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::device::Device;
use vulkano::sync::GpuFuture;

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::types::{ ByteStream, Version };
use crate::types::map::model::{ Model, Node, Node2, BoundingBox, ShadingType };
use crate::graphics::{ Transform, NativeModelVertex, Texture };
use crate::loaders::{ TextureLoader, GameFileLoader };
use crate::traits::ByteConvertable;

#[derive(Debug, ByteConvertable)]
pub struct PositionKeyframeData {
    pub frame: u32,
    pub position: Vector3<f32>,
}

#[derive(Debug, ByteConvertable)]
pub struct RotationKeyframeData {
    pub frame: u32,
    pub quaternions: Vector4<f32>,
}

#[allow(dead_code)]
#[derive(Debug, ByteConvertable)]
pub struct FaceData {
    pub vertex_position_indices: [u16; 3],
    pub texture_coordinate_indices: [u16; 3],
    pub texture_index: u16,
    pub padding: u16,
    pub two_sided: i32,
    pub smooth_group: i32,
}

#[derive(Debug, ByteConvertable)]
pub struct TextureCoordinateData {
    #[version_equals_or_above(1, 2)]
    pub color: Option<u32>,
    pub coordinates: Vector2<f32>, // possibly wrong if version < 1.2
}

#[derive(Debug, ByteConvertable)]
pub struct NodeData {
    #[length_hint(40)]
    pub node_name: String,
    #[length_hint(40)]
    pub parent_node_name: String,
    pub texture_count: u32,
    #[repeating(self.texture_count)]
    pub texture_indices: Vec<u32>,
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
    #[version_equals_or_above(1, 5)] // is this actually the right version?
    pub position_keyframe_count: Option<u32>,
    #[repeating(self.position_keyframe_count.unwrap_or_default())]
    pub position_keyframes: Vec<PositionKeyframeData>,
    pub rotation_keyframe_count: u32,
    #[repeating(self.rotation_keyframe_count)]
    pub rotation_keyframes: Vec<RotationKeyframeData>,
}

#[derive(Debug, ByteConvertable)]
pub struct ModelData {
    //#[version]
    //pub version: Version,
    pub animation_length: u32,
    pub shade_type: u32,
    #[version_equals_or_above(1, 4)]
    pub alpha: Option<u8>,
    pub resorved: [u8; 16],
    pub texture_count: u32,
    #[repeating(self.texture_count)]
    #[length_hint(40)]
    pub texture_names: Vec<String>,
    #[length_hint(40)]
    pub root_node_name: String,
    pub node_count: u32,
    #[repeating(self.node_count)]
    pub nodes: Vec<NodeData>,
}

#[derive(new)]
pub struct ModelLoader {
    game_file_loader: Rc<RefCell<GameFileLoader>>,
    device: Arc<Device>,
    #[new(default)]
    cache: HashMap<String, Arc<Model>>,
}

impl ModelLoader {

    /*fn calculate_bounding_box(nodes: &Vec<Node>) -> BoundingBox {

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
    }*/


    fn make_vertices(node: &NodeData, main_matrix: &Matrix4<f32>) -> Vec<NativeModelVertex> {
        
        let mut native_vertices = Vec::new();
        
        for face in &node.faces {

            // collect into tiny vec instead ?
            let vertex_positions: Vec<Vector3<f32>> = face.vertex_position_indices
                .iter()
                .copied()
                .map(|index| node.vertex_positions[index as usize])
                .collect();

            let texture_coordinates = face.texture_coordinate_indices
                .iter()
                .copied()
                .map(|index| node.texture_coordinates[index as usize].coordinates);

            let normal = NativeModelVertex::calculate_normal(vertex_positions[0], vertex_positions[1], vertex_positions[2]);

            for (vertex_position, texture_coordinates) in vertex_positions.into_iter().zip(texture_coordinates) {
                let adjusted_position = Self::multiply_matrix4_and_vector3(main_matrix, vertex_position);
                native_vertices.push(NativeModelVertex::new(adjusted_position, normal, texture_coordinates, face.texture_index as i32));
            }
        }

        native_vertices
    }

    fn multiply_matrix4_and_vector3(matrix: &Matrix4<f32>, vector: Vector3<f32>) -> Vector3<f32> {
        let adjusted_vector = matrix * vector4!(vector, 1.0);
        vector3!(adjusted_vector.x, adjusted_vector.y, adjusted_vector.z)
    }

    fn calculate_matrices(node: &NodeData, parent_matrix: &Matrix4<f32>) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {

        let mut main = Matrix4::from(node.offset_matrix);

        main = main * Matrix4::from_translation(node.translation1);

        let scale_matrix = Matrix4::from_nonuniform_scale(node.scale.x, node.scale.y, node.scale.z);
        let rotation_matrix = Matrix4::from_axis_angle(node.rotation_axis, Rad(node.rotation_angle));
        let translation_matrix = Matrix4::from_translation(node.translation2);
 
        let mut transform = scale_matrix;

        //if (node.frames.len() == 0) {
            transform = transform * rotation_matrix; // always apply, until we do animation)
        //}

        transform = transform * translation_matrix;

        let mut box_transform = scale_matrix;

        box_transform = box_transform * rotation_matrix;
        box_transform = box_transform * translation_matrix;
        box_transform = box_transform * parent_matrix;

        (main, transform, box_transform)
    }

    fn process_node_mesh(device: Arc<Device>, current_node: &NodeData, nodes: &Vec<NodeData>, textures: &Vec<Texture>, parent_matrix: &Matrix4<f32>, root_node_name: &str) -> Node2 {

        let (main_matrix, transform_matrix, box_transform_matrix) = Self::calculate_matrices(current_node, parent_matrix);
        let vertices = Self::make_vertices(current_node, &main_matrix);

        let m = main_matrix * box_transform_matrix;
        let bounding_box = BoundingBox::new_new(current_node.vertex_positions.iter().map(|position| Self::multiply_matrix4_and_vector3(&m, *position)));
        
        let final_matrix = match current_node.node_name.as_str() == root_node_name {
            true => transform_matrix * Matrix4::from_translation(vector3!(bounding_box.center().x, bounding_box.biggest.y, bounding_box.center().z)), // cache the center call ?
            false => transform_matrix,
        };
        
        let node_textures = current_node.texture_indices
            .iter()
            .map(|index| *index as usize)
            .map(|index| textures[index].clone())
            .collect();

        let child_nodes = nodes
            .iter()
            .filter(|node| node.parent_node_name == current_node.node_name)
            .map(|node| Self::process_node_mesh(device.clone(), node, nodes, textures, &box_transform_matrix, root_node_name))
            .collect();

        let vertices = NativeModelVertex::to_vertices(vertices);
        let vertex_buffer = CpuAccessibleBuffer::from_iter(device, BufferUsage::all(), false, vertices.into_iter()).unwrap();

        Node2::new(final_matrix, vertex_buffer, node_textures, child_nodes)
    }

    fn load(&mut self, texture_loader: &mut TextureLoader, model_file: &str, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Result<Arc<Model>, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load rsm model from {}{}{}", MAGENTA, model_file, NONE));

        let bytes = self.game_file_loader.borrow_mut().get(&format!("data\\model\\{}", model_file))?;
        let mut byte_stream = ByteStream::new(&bytes);

        let magic = byte_stream.string(4);
        
        if &magic != "GRSM" {
            return Err(format!("failed to read magic number from {}", model_file));
        }

        let major = byte_stream.next();
        let minor = byte_stream.next();
        byte_stream.set_version(Version::new(major, minor));

        println!("{}", byte_stream.get_version());

        let model_data = ModelData::from_bytes(&mut byte_stream, None);


        //println!("{}", model_data.version);

        let textures = model_data.texture_names
            .iter()
            .map(|texture_name| texture_loader.get(&texture_name, texture_future).unwrap())
            .collect();

        let root_node = model_data.nodes
            .iter()
            .find(|node_data| node_data.node_name == model_data.root_node_name)
            .expect("failed to find main node");

        let root_node = Self::process_node_mesh(self.device.clone(), root_node, &model_data.nodes, &textures, &Matrix4::identity(), &model_data.root_node_name);





        /*println!("{:?}", model_data);


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

        let root_node_name = byte_stream.string(40);
        let node_count = byte_stream.integer32();

        let mut nodes = Vec::new();

        for _index in 0..node_count as usize {

            let node_name = byte_stream.string(40);
            let parent_name = byte_stream.string(40);

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
            let _rotation_angle = byte_stream.float32();
            let _rotation_axis = byte_stream.vector3();
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
        }

        // always 8 x 0x0 ?
        let _unknown = byte_stream.slice(8);

        #[cfg(feature = "debug")]
        byte_stream.assert_empty(model_file);

        let bounding_box = Self::calculate_bounding_box(&nodes);

        for node in nodes.clone().iter() { // fix ordering issue
            if let Some(parent_name) = &node.parent_name {
                let parent_node = nodes.iter_mut().find(|node| node.name == *parent_name).expect("failed to find parent node");
                parent_node.child_nodes.push(node.clone());
            }
        }

        let root_node = nodes.iter().find(|node| node.name == *root_node_name).expect("failed to find root node").clone(); // fix cloning issue
        */

        let model = Arc::new(Model::new(root_node));

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
