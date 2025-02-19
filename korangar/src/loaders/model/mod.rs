use std::sync::Arc;

use cgmath::{EuclideanSpace, Matrix4, Point3, Rad, SquareMatrix, Vector2, Vector3};
use derive_new::new;
use hashbrown::HashSet;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_util::collision::AABB;
use korangar_util::math::multiply_matrix4_and_point3;
use korangar_util::texture_atlas::AllocationId;
use korangar_util::FileLoader;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::model::{ModelData, NodeData};
use ragnarok_formats::version::InternalVersion;

use super::error::LoadError;
use super::{smooth_model_normals, FALLBACK_MODEL_FILE};
use crate::graphics::{Color, NativeModelVertex};
use crate::loaders::map::DeferredVertexGeneration;
use crate::loaders::{GameFileLoader, TextureAtlas, TextureAtlasEntry};
use crate::world::{Model, Node};

#[derive(new)]
pub struct ModelLoader {
    game_file_loader: Arc<GameFileLoader>,
}

impl ModelLoader {
    fn add_vertices(
        vertices: &mut [NativeModelVertex],
        vertex_positions: &[Point3<f32>],
        texture_coordinates: &[Vector2<f32>],
        smoothing_groups: &[i32; 3],
        texture_index: i32,
        reverse_vertices: bool,
        reverse_normal: bool,
    ) {
        let normal = match reverse_normal {
            true => NativeModelVertex::calculate_normal(vertex_positions[0], vertex_positions[1], vertex_positions[2]),
            false => NativeModelVertex::calculate_normal(vertex_positions[2], vertex_positions[1], vertex_positions[0]),
        };

        if reverse_vertices {
            for ((vertex_position, texture_coordinates), target) in vertex_positions
                .iter()
                .zip(texture_coordinates.iter())
                .rev()
                .zip(vertices.iter_mut())
            {
                *target = NativeModelVertex::new(
                    *vertex_position,
                    normal,
                    *texture_coordinates,
                    texture_index,
                    Color::WHITE,
                    0.0, // TODO: actually add wind affinity
                    *smoothing_groups,
                );
            }
        } else {
            for ((vertex_position, texture_coordinates), target) in
                vertex_positions.iter().zip(texture_coordinates.iter()).zip(vertices.iter_mut())
            {
                *target = NativeModelVertex::new(
                    *vertex_position,
                    normal,
                    *texture_coordinates,
                    texture_index,
                    Color::WHITE,
                    0.0, // TODO: actually add wind affinity
                    *smoothing_groups,
                );
            }
        }
    }

    fn make_vertices(node: &NodeData, main_matrix: &Matrix4<f32>, reverse_order: bool, smooth_normals: bool) -> Vec<NativeModelVertex> {
        let face_count = node.faces.len();
        let face_vertex_count = face_count * 3;
        let two_sided_face_count = node.faces.iter().filter(|face| face.two_sided != 0).count();
        let total_vertices = (face_count + two_sided_face_count) * 3;

        let mut native_vertices = vec![NativeModelVertex::zeroed(); total_vertices];
        let mut face_index = 0;
        let mut back_face_index = face_vertex_count;

        let array: [f32; 3] = node.scale.unwrap().into();
        let reverse_node_order = array.into_iter().fold(1.0, |a, b| a * b).is_sign_negative();

        if reverse_node_order {
            panic!("this can actually happen");
        }

        for face in &node.faces {
            let vertex_positions: [Point3<f32>; 3] = std::array::from_fn(|index| {
                let position_index = face.vertex_position_indices[index];
                let position = node.vertex_positions[position_index as usize];
                multiply_matrix4_and_point3(main_matrix, position)
            });

            let texture_coordinates: [Vector2<f32>; 3] = std::array::from_fn(|index| {
                let coordinate_index = face.texture_coordinate_indices[index];
                node.texture_coordinates[coordinate_index as usize].coordinates
            });

            let smoothing_groups = match face.smooth_group_extra.as_ref() {
                None => [face.smooth_group, -1, -1],
                Some(extras) if extras.len() == 1 => [face.smooth_group, extras[0], -1],
                Some(extras) if extras.len() == 2 => [face.smooth_group, extras[0], extras[1]],
                _ => panic!("more than three smoothing groups found"),
            };

            Self::add_vertices(
                &mut native_vertices[face_index..face_index + 3],
                &vertex_positions,
                &texture_coordinates,
                &smoothing_groups,
                face.texture_index as i32,
                reverse_order,
                false,
            );
            face_index += 3;

            if face.two_sided != 0 {
                Self::add_vertices(
                    &mut native_vertices[back_face_index..back_face_index + 3],
                    &vertex_positions,
                    &texture_coordinates,
                    &smoothing_groups,
                    face.texture_index as i32,
                    !reverse_order,
                    true,
                );
                back_face_index += 3;
            }
        }

        if smooth_normals {
            let (face_vertices, back_face_vertices) = native_vertices.split_at_mut(face_vertex_count);
            smooth_model_normals(face_vertices);
            smooth_model_normals(back_face_vertices);
        }

        native_vertices
    }

    fn calculate_matrices(node: &NodeData, parent_matrix: &Matrix4<f32>) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
        let main = Matrix4::from_translation(node.translation1.unwrap()) * Matrix4::from(node.offset_matrix);
        let scale = node.scale.unwrap();
        let scale_matrix = Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
        let rotation_matrix = Matrix4::from_axis_angle(node.rotation_axis.unwrap(), Rad(node.rotation_angle.unwrap()));
        let translation_matrix = Matrix4::from_translation(node.translation2);

        let transform = match node.rotation_keyframe_count > 0 {
            true => translation_matrix * scale_matrix,
            false => translation_matrix * rotation_matrix * scale_matrix,
        };

        let box_transform = parent_matrix * translation_matrix * rotation_matrix * scale_matrix;

        (main, transform, box_transform)
    }

    fn calculate_centroid(vertices: &[NativeModelVertex]) -> Point3<f32> {
        let sum = vertices.iter().fold(Vector3::new(0.0, 0.0, 0.0), |accumulator, vertex| {
            accumulator + vertex.position.to_vec()
        });
        Point3::from_vec(sum / vertices.len() as f32)
    }

    fn process_node_mesh(
        current_node: &NodeData,
        nodes: &[NodeData],
        processed_node_indices: &mut [bool],
        vertex_offset: &mut usize,
        native_vertices: &mut Vec<NativeModelVertex>,
        model_texture_mapping: &[ModelTexture],
        parent_matrix: &Matrix4<f32>,
        main_bounding_box: &mut AABB,
        reverse_order: bool,
        smooth_normals: bool,
    ) -> Node {
        let (main_matrix, transform_matrix, box_transform_matrix) = Self::calculate_matrices(current_node, parent_matrix);

        let box_matrix = box_transform_matrix * main_matrix;
        let bounding_box = AABB::from_vertices(
            current_node
                .vertex_positions
                .iter()
                .map(|position| multiply_matrix4_and_point3(&box_matrix, *position)),
        );
        main_bounding_box.extend(&bounding_box);

        let child_indices: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|&(index, node)| {
                node.parent_node_name == current_node.node_name && !std::mem::replace(&mut processed_node_indices[index], true)
            })
            .map(|(i, _)| i)
            .collect();

        let child_nodes: Vec<Node> = child_indices
            .iter()
            .map(|&index| {
                Self::process_node_mesh(
                    &nodes[index],
                    nodes,
                    processed_node_indices,
                    vertex_offset,
                    native_vertices,
                    model_texture_mapping,
                    &box_transform_matrix,
                    main_bounding_box,
                    reverse_order,
                    smooth_normals,
                )
            })
            .collect();

        // Map the node texture index to the model texture index.
        let (node_texture_mapping, texture_transparency): (Vec<i32>, Vec<bool>) = current_node
            .texture_indices
            .iter()
            .map(|&index| {
                let model_texture = model_texture_mapping[index as usize];
                (model_texture.index, model_texture.transparent)
            })
            .unzip();

        let has_transparent_parts = texture_transparency.iter().any(|x| *x);

        let mut node_native_vertices = Self::make_vertices(current_node, &main_matrix, reverse_order, smooth_normals);
        let centroid = Self::calculate_centroid(&node_native_vertices);

        node_native_vertices
            .iter_mut()
            .for_each(|vertice| vertice.texture_index = node_texture_mapping[vertice.texture_index as usize]);

        // Remember the vertex offset/count and gather node vertices.
        let node_vertex_offset = *vertex_offset;
        let node_vertex_count = node_native_vertices.len();
        *vertex_offset += node_vertex_count;
        native_vertices.extend(node_native_vertices.iter());

        Node::new(
            transform_matrix,
            centroid,
            has_transparent_parts,
            node_vertex_offset,
            node_vertex_count,
            child_nodes,
            current_node.rotation_keyframes.clone(),
        )
    }

    pub fn calculate_transformation_matrix(node: &mut Node, is_root: bool, bounding_box: AABB, parent_matrix: Matrix4<f32>) {
        node.transform_matrix = match is_root {
            true => {
                let translation_matrix = Matrix4::from_translation(-Vector3::new(
                    bounding_box.center().x,
                    bounding_box.max().y,
                    bounding_box.center().z,
                ));

                parent_matrix * translation_matrix * node.transform_matrix
            }
            false => parent_matrix * node.transform_matrix,
        };

        node.child_nodes
            .iter_mut()
            .for_each(|child_node| Self::calculate_transformation_matrix(child_node, false, bounding_box, node.transform_matrix));
    }

    pub fn collect_model_textures(&self, textures: &mut HashSet<String>, model_file: &str) {
        let Ok(bytes) = self.game_file_loader.get(&format!("data\\model\\{model_file}")) else {
            return;
        };
        let mut byte_reader: ByteReader<Option<InternalVersion>> = ByteReader::with_default_metadata(&bytes);

        let Ok(model_data) = ModelData::from_bytes(&mut byte_reader) else {
            return;
        };

        model_data.texture_names.iter().for_each(|texture_name| {
            let _ = textures.insert(texture_name.inner.clone());
        });
    }

    pub fn load(
        &self,
        texture_atlas: &mut dyn TextureAtlas,
        vertex_offset: &mut usize,
        model_file: &str,
        reverse_order: bool,
    ) -> Result<(Model, DeferredVertexGeneration), LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load rsm model from {}", model_file.magenta()));

        let bytes = match self.game_file_loader.get(&format!("data\\model\\{model_file}")) {
            Ok(bytes) => bytes,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load model: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.load(texture_atlas, vertex_offset, FALLBACK_MODEL_FILE, reverse_order);
            }
        };
        let mut byte_reader: ByteReader<Option<InternalVersion>> = ByteReader::with_default_metadata(&bytes);

        let model_data = match ModelData::from_bytes(&mut byte_reader) {
            Ok(model_data) => model_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load model: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.load(texture_atlas, vertex_offset, FALLBACK_MODEL_FILE, reverse_order);
            }
        };

        // TODO: Temporary check until we support more versions.
        // TODO: The model operation to scale keyframe is not implemented yet.
        // TODO: The model operation to translate keyframe is not implemented yet.
        // TODO: The model operation to modify texture keyframe is not implemented yet.
        let version: InternalVersion = model_data.version.into();
        if version.equals_or_above(2, 2) {
            #[cfg(feature = "debug")]
            {
                print_debug!("Failed to load model because version {} is unsupported", version);
                print_debug!("Replacing with fallback");
            }

            return self.load(texture_atlas, vertex_offset, FALLBACK_MODEL_FILE, reverse_order);
        }

        let texture_allocation: Vec<TextureAtlasEntry> = model_data
            .texture_names
            .iter()
            .map(|texture_name| texture_atlas.register(texture_name.as_ref()))
            .collect();

        let texture_mapping: Vec<ModelTexture> = texture_allocation
            .iter()
            .enumerate()
            .map(|(index, entry)| ModelTexture {
                index: index as i32,
                transparent: entry.transparent,
            })
            .collect();

        let root_node_name = &model_data.root_node_name.clone().unwrap();

        let (root_node_position, root_node) = model_data
            .nodes
            .iter()
            .enumerate()
            .find(|(_, node_data)| &node_data.node_name == root_node_name)
            .expect("failed to find main node");

        let mut processed_node_indices = vec![false; model_data.nodes.len()];
        processed_node_indices[root_node_position] = true;

        let mut native_model_vertices = Vec::<NativeModelVertex>::new();

        let mut bounding_box = AABB::uninitialized();
        let root_node = Self::process_node_mesh(
            root_node,
            &model_data.nodes,
            &mut processed_node_indices,
            vertex_offset,
            &mut native_model_vertices,
            &texture_mapping,
            &Matrix4::identity(),
            &mut bounding_box,
            reverse_order,
            model_data.shade_type == 2,
        );
        let mut root_nodes = vec![root_node];

        for root_node in root_nodes.iter_mut() {
            Self::calculate_transformation_matrix(root_node, true, bounding_box, Matrix4::identity());
        }

        let model = Model::new(
            root_nodes,
            bounding_box,
            #[cfg(feature = "debug")]
            model_data,
        );

        let texture_allocation: Vec<AllocationId> = texture_allocation.iter().map(|entry| entry.allocation_id).collect();

        let deferred = DeferredVertexGeneration {
            native_model_vertices,
            texture_allocation,
        };

        #[cfg(feature = "debug")]
        timer.stop();

        Ok((model, deferred))
    }
}

#[derive(Copy, Clone)]
struct ModelTexture {
    index: i32,
    transparent: bool,
}
