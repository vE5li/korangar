mod vertices;
mod water_plane;

use std::sync::{Arc, Mutex};

use bytemuck::Pod;
use cgmath::Vector3;
use hashbrown::HashMap;
use korangar_audio::AudioEngine;
#[cfg(feature = "debug")]
use korangar_debug::logging::Timer;
use korangar_util::FileLoader;
use korangar_util::collision::{AABB, KDTree, Sphere};
use korangar_util::container::SimpleSlab;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::map::{GatData, GroundData, MapData, MapResources};
use ragnarok_formats::version::InternalVersion;
use wgpu::{BufferUsages, Device, Queue};

use self::vertices::{generate_tile_vertices, ground_vertices};
use self::water_plane::generate_water_plane;
use super::error::LoadError;
use crate::graphics::{BindlessSupport, Buffer, ModelVertex, TextureSet};
use crate::loaders::{GameFileLoader, ModelLoader, TextureLoader, TextureSetBuilder, VideoLoader, split_mesh_by_texture};
use crate::world::{Library, LightSourceKey, Lighting, Model, SubMesh, Video};
use crate::{EffectSourceExt, LightSourceExt, Map, Object, ObjectKey, SoundSourceExt};

pub const GROUND_TILE_SIZE: f32 = 10.0;
pub const GAT_TILE_SIZE: f32 = 5.0;

#[cfg(feature = "debug")]
fn assert_byte_reader_empty<Meta>(mut byte_reader: ByteReader<Meta>, file_name: &str) {
    use korangar_debug::logging::{Colorize, print_debug};

    if !byte_reader.is_empty() {
        print_debug!(
            "incomplete read on file {}; {} bytes remaining",
            file_name.magenta(),
            byte_reader.remaining_bytes().len().yellow(),
        );
    }
}

pub struct MapLoader {
    device: Device,
    queue: Queue,
    game_file_loader: Arc<GameFileLoader>,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    bindless_support: BindlessSupport,
}

impl MapLoader {
    pub fn new(
        device: Device,
        queue: Queue,
        game_file_loader: Arc<GameFileLoader>,
        audio_engine: Arc<AudioEngine<GameFileLoader>>,
        bindless_support: BindlessSupport,
    ) -> Self {
        Self {
            device,
            queue,
            game_file_loader,
            audio_engine,
            bindless_support,
        }
    }
}

impl MapLoader {
    pub fn load(
        &self,
        resource_file: String,
        model_loader: &ModelLoader,
        texture_loader: Arc<TextureLoader>,
        video_loader: Arc<VideoLoader>,
        library: &Library,
    ) -> Result<Box<Map>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load map from {}", &resource_file));

        let mut texture_set_builder = TextureSetBuilder::new(texture_loader.clone(), video_loader, resource_file.clone());

        let map_file_name = format!("data\\{}.rsw", &resource_file);
        let mut map_data: MapData = parse_generic_data(&map_file_name, &self.game_file_loader)?;

        // TODO: NHA Implement sky rendering
        let _map_sky_data = library.get_map_sky_data_from_resource_file(&resource_file);

        let ground_file = format!("data\\{}", map_data.ground_file);
        let ground_data: GroundData = parse_generic_data(&ground_file, &self.game_file_loader)?;

        let gat_file = format!("data\\{}", map_data.gat_file);
        let mut gat_data: GatData = parse_generic_data(&gat_file, &self.game_file_loader)?;

        #[cfg(feature = "debug")]
        let map_data_clone = map_data.clone();

        #[cfg(feature = "debug")]
        let (tile_vertices, mut tile_indices, tile_picker_vertices, tile_picker_indices) = generate_tile_vertices(&mut gat_data);

        #[cfg(not(feature = "debug"))]
        let (_, _, tile_picker_vertices, tile_picker_indices) = generate_tile_vertices(&mut gat_data);

        let (mut model_vertices, mut model_indices, ground_textures) = ground_vertices(&ground_data, &mut texture_set_builder);

        // TODO: NHA Support reading water planes from GND files (version >= 2.6).
        let water_plane = generate_water_plane(
            &self.device,
            &self.queue,
            &resource_file,
            &texture_loader,
            &ground_data,
            map_data.water_settings.as_ref(),
        );

        let sub_meshes = match self.bindless_support {
            BindlessSupport::Full | BindlessSupport::Limited => {
                vec![SubMesh {
                    index_offset: 0,
                    index_count: model_indices.len() as u32,
                    base_vertex: 0,
                    texture_index: 0,
                    transparent: false,
                }]
            }
            BindlessSupport::None => {
                let ground_texture_transparencies = ground_textures
                    .iter()
                    .map(|textures| (textures.index, textures.is_transparent))
                    .collect();
                split_mesh_by_texture(
                    &model_vertices,
                    &mut model_indices,
                    None,
                    None,
                    Some(&ground_texture_transparencies),
                )
            }
        };

        #[cfg(feature = "debug")]
        let tile_submeshes = match self.bindless_support {
            BindlessSupport::Full | BindlessSupport::Limited => {
                vec![SubMesh {
                    index_offset: 0,
                    index_count: tile_indices.len() as u32,
                    base_vertex: 0,
                    texture_index: 0,
                    transparent: true,
                }]
            }
            BindlessSupport::None => split_mesh_by_texture(&tile_vertices, &mut tile_indices, None, None, None),
        };

        #[cfg(feature = "debug")]
        let tile_vertex_buffer = Arc::new(create_vertex_buffer(
            &self.device,
            &self.queue,
            &resource_file,
            "tile vertex",
            &tile_vertices,
        ));

        #[cfg(feature = "debug")]
        let tile_index_buffer = Arc::new(create_index_buffer(
            &self.device,
            &self.queue,
            &resource_file,
            "tile index ",
            &tile_indices,
        ));

        let tile_picker_vertex_buffer = (!tile_picker_vertices.is_empty()).then(|| {
            create_vertex_buffer(
                &self.device,
                &self.queue,
                &resource_file,
                "tile picker vertex",
                &tile_picker_vertices,
            )
        });

        let tile_picker_index_buffer = (!tile_picker_indices.is_empty()).then(|| {
            create_index_buffer(
                &self.device,
                &self.queue,
                &resource_file,
                "tile picker index",
                &tile_picker_indices,
            )
        });

        apply_map_offset(&ground_data, &mut map_data.resources);

        let mut model_cache = HashMap::<(String, bool), Arc<Model>>::new();
        let mut objects = SimpleSlab::with_capacity(map_data.resources.objects.len() as u32);

        let object_bounding_boxes: Vec<(ObjectKey, AABB)> = map_data
            .resources
            .objects
            .iter()
            .map(|object_data| {
                let array: [f32; 3] = object_data.transform.scale.into();
                let reverse_order = array.into_iter().fold(1.0, |a, b| a * b).is_sign_negative();

                let model = model_cache
                    .entry((object_data.model_name.clone(), reverse_order))
                    .or_insert_with(|| {
                        Arc::new(
                            model_loader
                                .load(
                                    &mut texture_set_builder,
                                    &mut model_vertices,
                                    &mut model_indices,
                                    object_data.model_name.as_str(),
                                    reverse_order,
                                )
                                .expect("can't find model"),
                        )
                    })
                    .clone();

                let object = Object::new(
                    object_data.name.to_owned(),
                    object_data.model_name.to_owned(),
                    model,
                    object_data.transform,
                );
                let bounding_box = object.calculate_object_aabb();
                let key = objects.insert(object).expect("objects slab is full");

                (key, bounding_box)
            })
            .collect();
        let object_kdtree = KDTree::from_objects(&object_bounding_boxes);

        let BufferAndTextures {
            vertex_buffer,
            index_buffer,
            texture_set,
            videos,
        } = self.build_buffer_and_textures(&resource_file, texture_set_builder, model_vertices, model_indices);

        let lighting = Lighting::new(map_data.light_settings);

        let mut light_sources = SimpleSlab::with_capacity(map_data.resources.light_sources.len() as u32);
        let light_source_spheres: Vec<(LightSourceKey, Sphere)> = map_data
            .resources
            .light_sources
            .drain(..)
            .map(|light_source| {
                let sphere = Sphere::new(light_source.position, light_source.range);
                let key = light_sources.insert(light_source).expect("light sources slab is full");
                (key, sphere)
            })
            .collect();
        let light_sources_kdtree = KDTree::from_objects(&light_source_spheres);
        let background_music_track_name = self.audio_engine.get_track_for_map(&map_file_name);

        let map = Map::new(
            gat_data.map_width as usize,
            gat_data.map_height as usize,
            object_kdtree.root_boundary(),
            lighting,
            water_plane,
            gat_data.tiles,
            sub_meshes,
            vertex_buffer,
            index_buffer,
            texture_set,
            objects,
            light_sources,
            map_data.resources.sound_sources,
            #[cfg(feature = "debug")]
            map_data.resources.effect_sources,
            tile_picker_vertex_buffer.unwrap(),
            tile_picker_index_buffer.unwrap(),
            #[cfg(feature = "debug")]
            tile_vertex_buffer,
            #[cfg(feature = "debug")]
            tile_index_buffer,
            #[cfg(feature = "debug")]
            tile_submeshes,
            object_kdtree,
            light_sources_kdtree,
            background_music_track_name,
            videos,
            #[cfg(feature = "debug")]
            map_data_clone,
        );

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(Box::new(map))
    }

    fn build_buffer_and_textures(
        &self,
        resource_file: &str,
        texture_set_builder: TextureSetBuilder,
        model_vertices: Vec<ModelVertex>,
        model_indices: Vec<u32>,
    ) -> BufferAndTextures {
        let vertex_buffer = Arc::new(create_vertex_buffer(
            &self.device,
            &self.queue,
            resource_file,
            "map vertices",
            &model_vertices,
        ));
        let index_buffer = Arc::new(create_index_buffer(
            &self.device,
            &self.queue,
            resource_file,
            "map indices",
            &model_indices,
        ));
        let (texture_set, videos) = texture_set_builder.build();
        let texture_set = Arc::new(texture_set);
        let videos = Mutex::new(videos);

        BufferAndTextures {
            vertex_buffer,
            index_buffer,
            texture_set,
            videos,
        }
    }
}

struct BufferAndTextures {
    vertex_buffer: Arc<Buffer<ModelVertex>>,
    index_buffer: Arc<Buffer<u32>>,
    texture_set: Arc<TextureSet>,
    videos: Mutex<Vec<Video>>,
}

/// We shift the map resources, so that the world coordinate system's origin has
/// the same origin as the tile grids.
fn apply_map_offset(ground_data: &GroundData, resources: &mut MapResources) {
    let offset = Vector3::new(
        (ground_data.width as f32 * GROUND_TILE_SIZE) / 2.0,
        0.0,
        (ground_data.height as f32 * GROUND_TILE_SIZE) / 2.0,
    );

    resources.objects.iter_mut().for_each(|object| object.transform.position += offset);
    resources
        .sound_sources
        .iter_mut()
        .for_each(|sound_source| sound_source.offset(offset));
    resources
        .light_sources
        .iter_mut()
        .for_each(|light_source| light_source.offset(offset));
    resources
        .effect_sources
        .iter_mut()
        .for_each(|effect_source| effect_source.offset(offset));
}

fn parse_generic_data<Data: FromBytes>(resource_file: &str, game_file_loader: &GameFileLoader) -> Result<Data, LoadError> {
    let bytes = game_file_loader.get(resource_file).map_err(LoadError::File)?;
    let mut byte_reader: ByteReader<Option<InternalVersion>> = ByteReader::with_default_metadata(&bytes);

    let data = Data::from_bytes(&mut byte_reader).map_err(LoadError::Conversion)?;

    #[cfg(feature = "debug")]
    assert_byte_reader_empty(byte_reader, resource_file);

    Ok(data)
}

fn create_vertex_buffer<T: Pod>(device: &Device, queue: &Queue, resource: &str, label: &str, vertices: &[T]) -> Buffer<T> {
    Buffer::with_data(
        device,
        queue,
        format!("{resource} {label}"),
        BufferUsages::COPY_DST | BufferUsages::VERTEX,
        vertices,
    )
}

fn create_index_buffer(device: &Device, queue: &Queue, resource: &str, label: &str, indices: &[u32]) -> Buffer<u32> {
    Buffer::with_data(
        device,
        queue,
        format!("{resource} {label}"),
        BufferUsages::COPY_DST | BufferUsages::INDEX,
        indices,
    )
}
