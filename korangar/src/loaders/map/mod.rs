mod vertices;

use std::sync::Arc;

use bytemuck::Pod;
use cgmath::Vector3;
use derive_new::new;
use hashbrown::HashMap;
use korangar_audio::AudioEngine;
#[cfg(feature = "debug")]
use korangar_debug::logging::Timer;
use korangar_util::collision::{KDTree, Sphere, AABB};
use korangar_util::container::SimpleSlab;
use korangar_util::texture_atlas::{AllocationId, AtlasAllocation};
use korangar_util::FileLoader;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::map::{GatData, GroundData, MapData, MapResources};
use ragnarok_formats::version::InternalVersion;
use wgpu::{BufferUsages, Device, Queue};

pub use self::vertices::MAP_TILE_SIZE;
use self::vertices::{generate_tile_vertices, ground_vertices};
use super::error::LoadError;
use crate::graphics::{Buffer, ModelVertex, NativeModelVertex, Texture};
use crate::loaders::{GameFileLoader, ModelLoader, TextureAtlasFactory, TextureLoader};
use crate::world::{LightSourceKey, Model};
use crate::{EffectSourceExt, LightSourceExt, Map, Object, ObjectKey, SoundSourceExt};

const MAP_OFFSET: f32 = 5.0;

#[cfg(feature = "debug")]
fn assert_byte_reader_empty<Meta>(mut byte_reader: ByteReader<Meta>, file_name: &str) {
    use korangar_debug::logging::{print_debug, Colorize};

    if !byte_reader.is_empty() {
        print_debug!(
            "incomplete read on file {}; {} bytes remaining",
            file_name.magenta(),
            byte_reader.remaining_bytes().len().yellow(),
        );
    }
}

pub struct DeferredVertexGeneration {
    pub native_model_vertices: Vec<NativeModelVertex>,
    pub texture_allocation: Vec<AllocationId>,
}

#[derive(new)]
pub struct MapLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,
    game_file_loader: Arc<GameFileLoader>,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
}

impl MapLoader {
    pub fn load(
        &mut self,
        resource_file: String,
        model_loader: &mut ModelLoader,
        texture_loader: Arc<TextureLoader>,
        #[cfg(feature = "debug")] tile_texture_mapping: &[AtlasAllocation],
    ) -> Result<Map, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load map from {}", &resource_file));

        let mut texture_atlas_factory = TextureAtlasFactory::new(texture_loader.clone(), "map", true, true);
        let mut deferred_vertex_generation: Vec<DeferredVertexGeneration> = Vec::new();

        let map_file_name = format!("data\\{}.rsw", resource_file);
        let mut map_data: MapData = parse_generic_data(&map_file_name, &self.game_file_loader)?;

        let ground_file = format!("data\\{}", map_data.ground_file);
        let ground_data: GroundData = parse_generic_data(&ground_file, &self.game_file_loader)?;

        let gat_file = format!("data\\{}", map_data.gat_file);
        let mut gat_data: GatData = parse_generic_data(&gat_file, &self.game_file_loader)?;

        #[cfg(feature = "debug")]
        let map_data_clone = map_data.clone();

        #[cfg(feature = "debug")]
        let (tile_vertices, tile_picker_vertices) = generate_tile_vertices(&mut gat_data, tile_texture_mapping);
        #[cfg(not(feature = "debug"))]
        let (_, tile_picker_vertices) = generate_tile_vertices(&mut gat_data);

        let ground_native_vertices = ground_vertices(&ground_data);

        let ground_vertex_offset = 0;
        let ground_vertex_count = ground_native_vertices.len();
        let mut vertex_offset = ground_vertex_count;

        let ground_texture_allocation: Vec<AllocationId> = ground_data
            .textures
            .iter()
            .map(|texture_name| {
                let entry = texture_atlas_factory.register(texture_name);
                debug_assert!(!entry.transparent, "found transparent ground texture");
                entry.allocation_id
            })
            .collect();

        deferred_vertex_generation.push(DeferredVertexGeneration {
            native_model_vertices: ground_native_vertices,
            texture_allocation: ground_texture_allocation,
        });

        let water_textures: Option<Vec<Arc<Texture>>> =
            map_data
                .water_settings
                .as_ref()
                .and_then(|settings| settings.water_type)
                .map(|water_type| {
                    let water_paths = get_water_texture_paths(water_type);
                    water_paths
                        .iter()
                        .map(|path| texture_loader.get(path).expect("Can't load water texture"))
                        .collect()
                });

        #[cfg(feature = "debug")]
        let tile_vertex_buffer = Arc::new(
            (!tile_vertices.is_empty())
                .then(|| self.create_vertex_buffer(&resource_file, "tile", &tile_vertices))
                .unwrap(),
        );

        let tile_picker_vertex_buffer =
            (!tile_picker_vertices.is_empty()).then(|| self.create_vertex_buffer(&resource_file, "tile picker", &tile_picker_vertices));

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
                        let (model, deferred) = model_loader
                            .load(
                                &mut texture_atlas_factory,
                                &mut vertex_offset,
                                object_data.model_name.as_str(),
                                reverse_order,
                            )
                            .expect("can't find model");
                        deferred_vertex_generation.push(deferred);
                        Arc::new(model)
                    })
                    .clone();

                let object = Object::new(
                    object_data.name.to_owned(),
                    object_data.model_name.to_owned(),
                    model,
                    object_data.transform,
                );
                let bounding_box_matrix = object.get_bounding_box_matrix();
                let bounding_box = AABB::from_transformation_matrix(bounding_box_matrix);
                let key = objects.insert(object).expect("objects slab is full");

                (key, bounding_box)
            })
            .collect();
        let object_kdtree = KDTree::from_objects(&object_bounding_boxes);

        let (texture, vertex_buffer) =
            self.generate_vertex_buffer_and_atlas_texture(&resource_file, texture_atlas_factory, deferred_vertex_generation, vertex_offset);

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
            map_data.water_settings,
            map_data.light_settings,
            gat_data.tiles,
            ground_vertex_offset,
            ground_vertex_count,
            vertex_buffer,
            texture,
            water_textures,
            objects,
            light_sources,
            map_data.resources.sound_sources,
            #[cfg(feature = "debug")]
            map_data.resources.effect_sources,
            tile_picker_vertex_buffer.unwrap(),
            #[cfg(feature = "debug")]
            tile_vertex_buffer,
            object_kdtree,
            light_sources_kdtree,
            background_music_track_name,
            #[cfg(feature = "debug")]
            map_data_clone,
        );

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(map)
    }

    fn generate_vertex_buffer_and_atlas_texture(
        &mut self,
        resource_file: &str,
        mut texture_atlas_factory: TextureAtlasFactory,
        mut deferred_vertex_generation: Vec<DeferredVertexGeneration>,
        vertex_offset: usize,
    ) -> (Arc<Texture>, Arc<Buffer<ModelVertex>>) {
        // We can now generate the final texture atlas. Then we can map the final model
        // vertices texture coordinates.
        texture_atlas_factory.build_atlas();

        let mut vertices = Vec::with_capacity(vertex_offset);
        for mut deferred in deferred_vertex_generation.drain(..) {
            let texture_mapping: Vec<AtlasAllocation> = deferred
                .texture_allocation
                .drain(..)
                .map(|allocation_id| texture_atlas_factory.get_allocation(allocation_id).unwrap())
                .collect();
            let model_vertices = NativeModelVertex::to_vertices(deferred.native_model_vertices, &texture_mapping);
            vertices.extend(model_vertices);
        }

        let texture = texture_atlas_factory.upload_texture_atlas_texture();
        let vertex_buffer = Arc::new(self.create_vertex_buffer(resource_file, "map vertices", &vertices));

        (texture, vertex_buffer)
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
        let filename = format!("¿öÅÍ\\water{}{:02}.jpg", water_type, i);
        paths.push(filename);
    }
    paths
}
