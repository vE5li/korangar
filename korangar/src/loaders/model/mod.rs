use std::sync::Arc;

use cgmath::{Array, EuclideanSpace, Matrix4, Point3, Rad, SquareMatrix, Vector2, Vector3};
use hashbrown::{HashMap, HashSet};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_util::FileLoader;
use korangar_util::collision::AABB;
use korangar_util::math::multiply_matrix4_and_point3;
use num::Zero;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::model::{ModelData, NodeData};
use ragnarok_formats::version::InternalVersion;
use smallvec::SmallVec;

use super::error::LoadError;
use super::{FALLBACK_MODEL_FILE, TextureSetBuilder, TextureSetTexture, smooth_model_normals};
use crate::graphics::{BindlessSupport, Color, ModelVertex, NativeModelVertex, reduce_vertices};
use crate::loaders::GameFileLoader;
use crate::world::{Model, Node, SubMesh};

pub struct ModelLoader {
    game_file_loader: Arc<GameFileLoader>,
    bindless_support: BindlessSupport,
}

impl ModelLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>, bindless_support: BindlessSupport) -> Self {
        Self {
            game_file_loader,
            bindless_support,
        }
    }
}

impl ModelLoader {
    fn add_vertices(
        vertices: &mut [NativeModelVertex],
        vertex_positions: &[Point3<f32>],
        texture_coordinates: &[Vector2<f32>],
        smoothing_groups: &SmallVec<[i32; 3]>,
        texture_index: i32,
        reverse_vertices: bool,
        reverse_normal: bool,
    ) {
        let normal = match reverse_normal {
            true => NativeModelVertex::calculate_normal(vertex_positions[0], vertex_positions[1], vertex_positions[2]),
            false => NativeModelVertex::calculate_normal(vertex_positions[2], vertex_positions[1], vertex_positions[0]),
        };

        // If there are degenerated triangles, we at least set a valid normal value.
        let normal = normal.unwrap_or_else(Vector3::unit_y);

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
                    smoothing_groups.clone(),
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
                    smoothing_groups.clone(),
                );
            }
        }
    }

    fn make_vertices(node: &NodeData, main_matrix: &Matrix4<f32>, reverse_order: bool, smooth_normals: bool) -> Vec<NativeModelVertex> {
        let face_count = node.faces.len();
        let face_vertex_count = face_count * 3;
        let two_sided_face_count = node.faces.iter().filter(|face| face.two_sided != 0).count();
        let total_vertices = (face_count + two_sided_face_count) * 3;

        let mut vertices = vec![NativeModelVertex::zeroed(); total_vertices];

        let mut face_index = 0;
        let mut back_face_index = face_vertex_count;

        let array: [f32; 3] = node.scale.unwrap_or(Vector3::new(1.0, 1.0, 1.0)).into();
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

            let smoothing_groups: SmallVec<[i32; 3]> = SmallVec::from_iter(
                std::iter::once(face.smooth_group).chain(face.smooth_group_extra.as_ref().iter().flat_map(|extra| extra.iter().copied())),
            );

            Self::add_vertices(
                &mut vertices[face_index..face_index + 3],
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
                    &mut vertices[back_face_index..back_face_index + 3],
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
            let (face_vertices, back_face_vertices) = vertices.split_at_mut(face_vertex_count);
            smooth_model_normals(face_vertices);
            smooth_model_normals(back_face_vertices);
        }

        vertices
    }

    fn calculate_matrices_rsm1(node: &NodeData, parent_matrix: &Matrix4<f32>) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
        let main = Matrix4::from_translation(node.translation1.unwrap_or(Vector3::zero())) * Matrix4::from(node.offset_matrix);
        let scale = node.scale.unwrap_or(Vector3::from_value(1.0));
        let scale_matrix = Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);
        let rotation_matrix = Matrix4::from_axis_angle(
            node.rotation_axis.unwrap_or(Vector3::zero()),
            Rad(node.rotation_angle.unwrap_or(0.0)),
        );
        let translation_matrix = Matrix4::from_translation(node.translation2);

        let transform = match node.rotation_keyframe_count > 0 {
            true => translation_matrix * scale_matrix,
            false => translation_matrix * rotation_matrix * scale_matrix,
        };

        let box_transform = parent_matrix * translation_matrix * rotation_matrix * scale_matrix;

        (main, transform, box_transform)
    }

    fn calculate_matrices_rsm2(node: &NodeData) -> (Matrix4<f32>, Matrix4<f32>, Matrix4<f32>) {
        let main = Matrix4::identity();
        let translation_matrix = Matrix4::from_translation(node.translation2);
        let transform = translation_matrix * Matrix4::from(node.offset_matrix);
        let box_transform = transform;

        (main, transform, box_transform)
    }

    fn calculate_centroid(vertices: &[NativeModelVertex]) -> Point3<f32> {
        let sum = vertices.iter().fold(Vector3::new(0.0, 0.0, 0.0), |accumulator, vertex| {
            accumulator + vertex.position.to_vec()
        });
        Point3::from_vec(sum / vertices.len() as f32)
    }

    fn process_node_mesh(
        bindless_support: BindlessSupport,
        version: InternalVersion,
        current_node: &NodeData,
        nodes: &[NodeData],
        processed_node_indices: &mut [bool],
        model_vertices: &mut Vec<ModelVertex>,
        model_indices: &mut Vec<u32>,
        texture_mapping: &TextureMapping,
        parent_matrix: &Matrix4<f32>,
        main_bounding_box: &mut AABB,
        reverse_order: bool,
        smooth_normals: bool,
        frames_per_second: f32,
        animation_length: u32,
    ) -> Node {
        let (main_matrix, transform_matrix, box_transform_matrix) = match version.equals_or_above(2, 2) {
            false => Self::calculate_matrices_rsm1(current_node, parent_matrix),
            true => Self::calculate_matrices_rsm2(current_node),
        };

        let rotation_matrix = current_node.offset_matrix;
        let position = current_node.translation2.extend(0.0);

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
                    bindless_support,
                    version,
                    &nodes[index],
                    nodes,
                    processed_node_indices,
                    model_vertices,
                    model_indices,
                    texture_mapping,
                    &box_transform_matrix,
                    main_bounding_box,
                    reverse_order,
                    smooth_normals,
                    frames_per_second,
                    animation_length,
                )
            })
            .collect();

        let node_textures: Vec<TextureSetTexture> = match texture_mapping {
            TextureMapping::PreVersion2_3(vector_texture) => current_node
                .texture_indices
                .iter()
                .map(|&index| vector_texture[index as usize])
                .collect(),
            TextureMapping::PostVersion2_3(hashmap_texture) => current_node
                .texture_names
                .iter()
                .map(|name| *hashmap_texture.get(name.as_ref()).unwrap())
                .collect(),
        };

        let node_native_vertices = Self::make_vertices(current_node, &main_matrix, reverse_order, smooth_normals);

        let centroid = Self::calculate_centroid(&node_native_vertices);

        let node_vertices = NativeModelVertex::convert_to_model_vertices(node_native_vertices, Some(&node_textures));
        let (node_vertices, mut node_indices) = reduce_vertices(&node_vertices);

        // Apply the frames per second on the keyframes values.
        let animation_length = match version.equals_or_above(2, 2) {
            true => (animation_length as f32 * 1000.0 / frames_per_second).floor() as u32,
            false => animation_length,
        };

        let scale_keyframes = match version.equals_or_above(2, 2) {
            true => {
                let mut scale_keyframes = current_node.scale_keyframes.clone();
                for data in scale_keyframes.iter_mut() {
                    data.frame = (data.frame as f32 * 1000.0 / frames_per_second).floor() as i32;
                }
                scale_keyframes
            }
            false => current_node.scale_keyframes.clone(),
        };

        let translation_keyframes = match version.equals_or_above(2, 2) {
            true => {
                let mut translation_keyframes = current_node.translation_keyframes.clone();
                for data in translation_keyframes.iter_mut() {
                    data.frame = (data.frame as f32 * 1000.0 / frames_per_second).floor() as i32;
                }
                translation_keyframes
            }
            false => current_node.translation_keyframes.clone(),
        };

        let rotation_keyframes = match version.equals_or_above(2, 2) {
            true => {
                let mut rotation_keyframes = current_node.rotation_keyframes.clone();
                for data in rotation_keyframes.iter_mut() {
                    data.frame = (data.frame as f32 * 1000.0 / frames_per_second).floor() as i32;
                }
                rotation_keyframes
            }
            false => current_node.rotation_keyframes.clone(),
        };

        match bindless_support {
            BindlessSupport::Full | BindlessSupport::Limited => {
                // Remember the index offset, index count, base vertex and gather node vertices.
                let index_offset = model_indices.len() as u32;
                let index_count = node_indices.len() as u32;
                let base_vertex = model_vertices.len() as i32;
                model_vertices.extend(node_vertices);
                model_indices.extend(node_indices);

                Node::new(
                    version,
                    transform_matrix,
                    rotation_matrix.into(),
                    Matrix4::identity(),
                    position,
                    centroid,
                    vec![SubMesh {
                        index_offset,
                        index_count,
                        base_vertex,
                        texture_index: 0,
                        transparent: node_textures.iter().any(|texture| texture.is_transparent),
                    }],
                    child_nodes,
                    animation_length,
                    scale_keyframes,
                    translation_keyframes,
                    rotation_keyframes,
                )
            }
            BindlessSupport::None => {
                let texture_transparencies: HashMap<i32, bool> = node_textures
                    .iter()
                    .map(|texture| (texture.index, texture.is_transparent))
                    .collect();

                let submeshes = split_mesh_by_texture(
                    &node_vertices,
                    &mut node_indices,
                    Some(model_vertices),
                    Some(model_indices),
                    Some(&texture_transparencies),
                );

                Node::new(
                    version,
                    transform_matrix,
                    rotation_matrix.into(),
                    Matrix4::identity(),
                    position,
                    centroid,
                    submeshes,
                    child_nodes,
                    animation_length,
                    scale_keyframes,
                    translation_keyframes,
                    rotation_keyframes,
                )
            }
        }
    }

    pub fn calculate_transformation_matrix(
        node: &mut Node,
        is_root: bool,
        bounding_box: AABB,
        parent_matrix: &Matrix4<f32>,
        parent_rotation_matrix: &Matrix4<f32>,
        is_static: bool,
    ) {
        let transform_matrix = match is_root {
            true => {
                let translation_matrix = Matrix4::from_translation(-Vector3::new(
                    bounding_box.center().x,
                    bounding_box.max().y,
                    bounding_box.center().z,
                ));
                match node.version.equals_or_above(2, 2) {
                    true => node.transform_matrix,
                    false => translation_matrix * node.transform_matrix,
                }
            }
            false => node.transform_matrix,
        };

        match node.version.equals_or_above(2, 2) {
            false => {
                node.transform_matrix = match is_static {
                    true => parent_matrix * transform_matrix,
                    false => transform_matrix,
                };
            }
            true => node.parent_rotation_matrix = *parent_rotation_matrix,
        }

        node.child_nodes.iter_mut().for_each(|child_node| {
            Self::calculate_transformation_matrix(
                child_node,
                false,
                bounding_box,
                &node.transform_matrix,
                &node.rotation_matrix,
                is_static,
            );
        });
    }

    // A model is static, if it doesn't have any animations.
    pub fn is_static(node: &Node) -> bool {
        node.scale_keyframes.is_empty()
            && node.translation_keyframes.is_empty()
            && node.rotation_keyframes.is_empty()
            && node.child_nodes.iter().all(Self::is_static)
    }

    /// We need to make sure to always generate a texture atlas in the same
    /// order when creating an online texture atlas and an offline texture
    /// atlas.
    fn collect_versioned_texture_names(version: &InternalVersion, model_data: &ModelData) -> Vec<String> {
        match version.equals_or_above(2, 3) {
            false => model_data
                .texture_names
                .iter()
                .map(|texture_name| texture_name.inner.clone())
                .collect(),
            true => {
                let mut hashset = HashSet::<String>::new();
                let mut result = Vec::<String>::with_capacity(5);
                model_data.nodes.iter().for_each(|node_data| {
                    node_data.texture_names.iter().for_each(|name| {
                        let inner_name = &name.inner;
                        if !hashset.contains(inner_name) {
                            hashset.insert(inner_name.clone());
                            result.push(name.inner.clone());
                        }
                    })
                });
                result
            }
        }
    }

    pub fn load(
        &self,
        texture_set_builder: &mut TextureSetBuilder,
        model_vertices: &mut Vec<ModelVertex>,
        model_indices: &mut Vec<u32>,
        model_file: &str,
        reverse_order: bool,
    ) -> Result<Model, LoadError> {
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

                return self.load(
                    texture_set_builder,
                    model_vertices,
                    model_indices,
                    FALLBACK_MODEL_FILE,
                    reverse_order,
                );
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

                return self.load(
                    texture_set_builder,
                    model_vertices,
                    model_indices,
                    FALLBACK_MODEL_FILE,
                    reverse_order,
                );
            }
        };

        // TODO: Temporary check until we support more versions.
        // TODO: The model operation to modify texture keyframe is not implemented yet.
        let version: InternalVersion = model_data.version.into();
        if version.equals_or_above(2, 4) {
            #[cfg(feature = "debug")]
            {
                print_debug!("Failed to load model because version {} is unsupported", version);
                print_debug!("Replacing with fallback");
            }

            return self.load(
                texture_set_builder,
                model_vertices,
                model_indices,
                FALLBACK_MODEL_FILE,
                reverse_order,
            );
        }

        let texture_names = ModelLoader::collect_versioned_texture_names(&version, &model_data);

        let model_textures: Vec<TextureSetTexture> = texture_names
            .iter()
            .map(|texture_name| texture_set_builder.register(texture_name.as_ref()))
            .collect();

        let texture_mapping = match version.equals_or_above(2, 3) {
            true => {
                let model_textures =
                    HashMap::<String, TextureSetTexture>::from_iter(texture_names.into_iter().zip(model_textures.iter().copied()));
                TextureMapping::PostVersion2_3(model_textures)
            }
            false => TextureMapping::PreVersion2_3(model_textures),
        };

        let root_node_names = match version.equals_or_above(2, 2) {
            true => model_data.root_node_names.to_vec(),
            false => vec![model_data.root_node_name.clone().unwrap()],
        };

        let root_info: Vec<(usize, &NodeData)> = root_node_names
            .iter()
            .map(|node_name| {
                let (root_node_position, root_node) = model_data
                    .nodes
                    .iter()
                    .enumerate()
                    .find(|(_, node_data)| node_data.node_name == *node_name)
                    .expect("failed to find main node");
                (root_node_position, root_node)
            })
            .collect();

        let mut processed_node_indices = vec![false; model_data.nodes.len()];
        let mut model_bounding_box = AABB::uninitialized();

        let mut root_nodes: Vec<Node> = root_info
            .into_iter()
            .map(|(root_node_position, root_node)| {
                processed_node_indices[root_node_position] = true;
                Self::process_node_mesh(
                    self.bindless_support,
                    version,
                    root_node,
                    &model_data.nodes,
                    &mut processed_node_indices,
                    model_vertices,
                    model_indices,
                    &texture_mapping,
                    &Matrix4::identity(),
                    &mut model_bounding_box,
                    reverse_order ^ version.equals_or_above(2, 2),
                    model_data.shade_type == 2,
                    model_data.frames_per_second.unwrap_or(60.0),
                    model_data.animation_length,
                )
            })
            .collect();

        drop(texture_mapping);

        let is_static = root_nodes.iter().all(Self::is_static);

        for root_node in root_nodes.iter_mut() {
            Self::calculate_transformation_matrix(
                root_node,
                true,
                model_bounding_box,
                &Matrix4::identity(),
                &Matrix4::identity(),
                is_static,
            );
        }

        let model = Model::new(
            version,
            root_nodes,
            model_bounding_box,
            is_static,
            #[cfg(feature = "debug")]
            model_data,
        );

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(model)
    }
}

/// When bindless is not supported, we need to create separate meshes for
/// each texture used in the mesh so we can bind the appropriate texture
/// before drawing each sub-mesh.
pub fn split_mesh_by_texture(
    vertices: &[ModelVertex],
    indices: &mut [u32],
    mut global_model_vertices: Option<&mut Vec<ModelVertex>>,
    mut global_model_indices: Option<&mut Vec<u32>>,
    texture_transparencies: Option<&HashMap<i32, bool>>,
) -> Vec<SubMesh> {
    let mut texture_to_faces: HashMap<i32, Vec<[u32; 3]>> = HashMap::new();

    indices.chunks(3).filter(|chunk| chunk.len() == 3).for_each(|chunk| {
        let texture_index = vertices[chunk[0] as usize].texture_index;
        texture_to_faces
            .entry(texture_index)
            .or_default()
            .push([chunk[0], chunk[1], chunk[2]])
    });

    let mut submeshes = Vec::new();

    match (global_model_vertices.as_mut(), global_model_indices.as_mut()) {
        (Some(global_vertices), Some(global_indices)) => {
            for (&texture_index, faces) in texture_to_faces.iter() {
                let transparent = texture_transparencies
                    .and_then(|transparencies| transparencies.get(&texture_index).copied())
                    .unwrap_or(false);

                let index_count = (faces.len() * 3) as u32;
                let index_offset = global_indices.len() as u32;

                let mut index_mapping = HashMap::new();
                let base_vertex = global_vertices.len() as i32;

                for &old_index in faces.iter().flatten() {
                    if !index_mapping.contains_key(&old_index) {
                        index_mapping.insert(old_index, index_mapping.len() as u32);
                        global_vertices.push(vertices[old_index as usize]);
                    }
                }

                for old_index in faces.iter().flatten() {
                    global_indices.push(index_mapping[old_index]);
                }

                submeshes.push(SubMesh {
                    index_offset,
                    index_count,
                    base_vertex,
                    texture_index,
                    transparent,
                });
            }
        }
        _ => {
            let mut current_index = 0;

            for (&texture_index, faces) in texture_to_faces.iter() {
                let transparent = texture_transparencies
                    .and_then(|transparencies| transparencies.get(&texture_index).copied())
                    .unwrap_or(false);

                let index_count = (faces.len() * 3) as u32;

                for (index, &vertex_index) in faces.iter().flatten().enumerate() {
                    indices[current_index + index] = vertex_index;
                }

                submeshes.push(SubMesh {
                    index_offset: current_index as u32,
                    index_count,
                    base_vertex: 0,
                    texture_index,
                    transparent,
                });

                current_index += index_count as usize;
            }
        }
    }

    submeshes
}

enum TextureMapping {
    PreVersion2_3(Vec<TextureSetTexture>),
    PostVersion2_3(HashMap<String, TextureSetTexture>),
}
