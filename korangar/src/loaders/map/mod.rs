mod vertices;

use std::sync::{Arc, Mutex};

use bytemuck::Pod;
use cgmath::{Array, Point2, Vector3};
use derive_new::new;
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

pub use self::vertices::MAP_TILE_SIZE;
use self::vertices::{generate_tile_vertices, ground_vertices};
use super::error::LoadError;
use crate::graphics::{BindlessSupport, Buffer, ModelVertex, Texture, TextureSet};
use crate::loaders::{GameFileLoader, ImageType, ModelLoader, TextureLoader, TextureSetBuilder, VideoLoader, split_mesh_by_texture};
use crate::world::{Library, LightSourceKey, Lighting, Model, SubMesh, Video};
use crate::{EffectSourceExt, LightSourceExt, Map, Object, ObjectKey, SoundSourceExt};

const MAP_OFFSET: f32 = 5.0;

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

#[derive(new)]
pub struct MapLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    game_file_loader: Arc<GameFileLoader>,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    bindless_support: BindlessSupport,
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

        let water_level = -map_data
            .water_settings
            .as_ref()
            .and_then(|settings| settings.water_level)
            .unwrap_or_default();

        let (mut model_vertices, mut model_indices, water_bounds, mut ground_texture_transparencies) =
            ground_vertices(&ground_data, water_level, &mut texture_set_builder);

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
                let ground_texture_transparencies = ground_texture_transparencies
                    .drain(..)
                    .enumerate()
                    .map(|(index, transparent)| (index as i32, transparent))
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

        let water_textures: Option<Vec<Arc<Texture>>> =
            map_data
                .water_settings
                .as_ref()
                .and_then(|settings| settings.water_type)
                .map(|water_type| {
                    let water_paths = get_water_texture_paths(water_type);
                    water_paths
                        .iter()
                        .map(|path| {
                            texture_loader
                                .get_or_load(path, ImageType::Color)
                                .expect("Can't load water texture")
                        })
                        .collect()
                });

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
        let tile_vertex_buffer = Arc::new(self.create_vertex_buffer(&resource_file, "tile vertex", &tile_vertices));

        #[cfg(feature = "debug")]
        let tile_index_buffer = Arc::new(self.create_index_buffer(&resource_file, "tile index ", &tile_indices));

        let tile_picker_vertex_buffer = (!tile_picker_vertices.is_empty())
            .then(|| self.create_vertex_buffer(&resource_file, "tile picker vertex", &tile_picker_vertices));

        let tile_picker_index_buffer =
            (!tile_picker_indices.is_empty()).then(|| self.create_index_buffer(&resource_file, "tile picker index", &tile_picker_indices));

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

        let (vertex_buffer, index_buffer, texture_set, videos) =
            self.build_buffer_and_texture_set_and_videos(&resource_file, texture_set_builder, model_vertices, model_indices);

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

        // There are maps that don't have water tiles but have water settings. In such
        // cases we will set the water settings to `None`
        let water_settings = map_data
            .water_settings
            .filter(|_| !(water_bounds.min == Point2::from_value(f32::MAX) && water_bounds.max == Point2::from_value(f32::MIN)));

        let map = Map::new(
            gat_data.map_width as usize,
            gat_data.map_height as usize,
            lighting,
            water_settings,
            water_bounds,
            gat_data.tiles,
            sub_meshes,
            vertex_buffer,
            index_buffer,
            texture_set,
            water_textures,
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

    fn build_buffer_and_texture_set_and_videos(
        &self,
        resource_file: &str,
        texture_set_builder: TextureSetBuilder,
        model_vertices: Vec<ModelVertex>,
        model_indices: Vec<u32>,
    ) -> (Arc<Buffer<ModelVertex>>, Arc<Buffer<u32>>, Arc<TextureSet>, Mutex<Vec<Video>>) {
        let vertex_buffer = Arc::new(self.create_vertex_buffer(resource_file, "map vertices", &model_vertices));
        let index_buffer = Arc::new(self.create_index_buffer(resource_file, "map indices", &model_indices));
        let (texture_set, videos) = texture_set_builder.build();
        let texture_set = Arc::new(texture_set);
        let videos = Mutex::new(videos);
        (vertex_buffer, index_buffer, texture_set, videos)
    }

    fn create_vertex_buffer<T: Pod>(&self, resource: &str, label: &str, vertices: &[T]) -> Buffer<T> {
        Buffer::with_data(
            &self.device,
            &self.queue,
            format!("{resource} {label}"),
            BufferUsages::COPY_DST | BufferUsages::VERTEX,
            vertices,
        )
    }

    fn create_index_buffer(&self, resource: &str, label: &str, indices: &[u32]) -> Buffer<u32> {
        Buffer::with_data(
            &self.device,
            &self.queue,
            format!("{resource} {label}"),
            BufferUsages::COPY_DST | BufferUsages::INDEX,
            indices,
        )
    }
}

fn apply_map_offset(ground_data: &GroundData, resources: &mut MapResources) {
    let offset = Vector3::new(
        ground_data.width as f32 * MAP_OFFSET,
        0.0,
        ground_data.height as f32 * MAP_OFFSET,
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

fn get_water_texture_paths(water_type: i32) -> Vec<String> {
    let mut paths = Vec::with_capacity(32);
    for i in 0..32 {
        let filename = format!("워터\\water{}{:02}.jpg", water_type, i);
        paths.push(filename);
    }
    paths
}
