mod vertices;

use std::collections::HashMap;
use std::sync::Arc;

use cgmath::Vector3;
use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::Timer;
use ragnarok_bytes::{ByteStream, FromBytes};
use ragnarok_formats::map::{GatData, GroundData, GroundTile, MapData, MapResources};
use ragnarok_formats::version::InternalVersion;

use self::vertices::{generate_tile_vertices, ground_water_vertices, load_textures};
use super::error::LoadError;
use crate::graphics::{BufferAllocator, NativeModelVertex};
use crate::loaders::{GameFileLoader, ModelLoader, TextureLoader};
use crate::world::*;

const MAP_OFFSET: f32 = 5.0;

#[cfg(feature = "debug")]
fn assert_byte_stream_empty<Meta>(mut byte_stream: ByteStream<Meta>, file_name: &str) {
    use korangar_debug::logging::{print_debug, Colorize};

    if byte_stream.is_empty() {
        print_debug!(
            "incomplete read on file {}; {} bytes remaining",
            file_name.magenta(),
            byte_stream.remaining_bytes().len().yellow(),
        );
    }
}

#[derive(new)]
pub struct MapLoader {
    #[new(default)]
    cache: HashMap<String, Arc<Map>>,
}

impl MapLoader {
    pub fn get(
        &mut self,
        resource_file: String,
        game_file_loader: &mut GameFileLoader,
        buffer_allocator: &mut BufferAllocator,
        model_loader: &mut ModelLoader,
        texture_loader: &mut TextureLoader,
    ) -> Result<Arc<Map>, LoadError> {
        match self.cache.get(&resource_file) {
            Some(map) => Ok(map.clone()),
            None => self.load(resource_file, game_file_loader, buffer_allocator, model_loader, texture_loader),
        }
    }

    fn load(
        &mut self,
        resource_file: String,
        game_file_loader: &mut GameFileLoader,
        buffer_allocator: &mut BufferAllocator,
        model_loader: &mut ModelLoader,
        texture_loader: &mut TextureLoader,
    ) -> Result<Arc<Map>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load map from {}", &resource_file));

        let map_file = format!("data\\{}.rsw", resource_file);
        let mut map_data: MapData = parse_generic_data(&map_file, game_file_loader)?;

        let ground_file = format!("data\\{}", map_data.ground_file);
        let ground_data: GroundData = parse_generic_data(&ground_file, game_file_loader)?;

        let gat_file = format!("data\\{}", map_data.gat_file);
        let mut gat_data: GatData = parse_generic_data(&gat_file, game_file_loader)?;

        #[cfg(feature = "debug")]
        let map_data_clone = map_data.clone();

        let (tile_vertices, tile_picker_vertices) = generate_tile_vertices(&mut gat_data);
        let water_level = -map_data
            .water_settings
            .as_ref()
            .and_then(|settings| settings.water_level)
            .unwrap_or_default();
        let (ground_vertices, water_vertices) = ground_water_vertices(&ground_data, water_level);

        let ground_vertices = NativeModelVertex::to_vertices(ground_vertices);
        let ground_vertex_buffer = buffer_allocator.allocate_vertex_buffer(ground_vertices);
        let water_vertex_buffer = (!water_vertices.is_empty()).then(|| buffer_allocator.allocate_vertex_buffer(water_vertices));
        let tile_vertex_buffer = (!tile_vertices.is_empty()).then(|| buffer_allocator.allocate_vertex_buffer(tile_vertices));
        let tile_picker_vertex_buffer =
            (!tile_picker_vertices.is_empty()).then(|| buffer_allocator.allocate_vertex_buffer(tile_picker_vertices));

        let textures = load_textures(&ground_data, texture_loader, game_file_loader);
        apply_map_offset(&ground_data, &mut map_data.resources);

        // Loading object models
        let objects: Vec<Object> = map_data
            .resources
            .objects
            .iter()
            .map(|object_data| {
                let array: [f32; 3] = object_data.transform.scale.into();
                let reverse_order = array.into_iter().fold(1.0, |a, b| a * b).is_sign_negative();
                let model = model_loader.get(
                    buffer_allocator,
                    game_file_loader,
                    texture_loader,
                    object_data.model_name.as_str(),
                    reverse_order,
                );

                Object::new(
                    object_data.name.to_owned(),
                    object_data.model_name.to_owned(),
                    model.unwrap(),
                    object_data.transform,
                )
            })
            .collect();

        let map = Arc::new(Map::new(
            gat_data.map_width as usize,
            gat_data.map_height as usize,
            map_data.water_settings,
            map_data.light_settings,
            gat_data.tiles,
            ground_vertex_buffer,
            water_vertex_buffer,
            textures,
            objects,
            map_data.resources.light_sources,
            map_data.resources.sound_sources,
            map_data.resources.effect_sources,
            tile_picker_vertex_buffer.unwrap(),
            tile_vertex_buffer.unwrap(),
            #[cfg(feature = "debug")]
            map_data_clone,
        ));

        self.cache.insert(resource_file, map.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(map)
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

fn parse_generic_data<Data: FromBytes>(resource_file: &str, game_file_loader: &mut GameFileLoader) -> Result<Data, LoadError> {
    let bytes = game_file_loader.get(resource_file).map_err(LoadError::File)?;
    let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

    let data = Data::from_bytes(&mut byte_stream).map_err(LoadError::Conversion)?;

    #[cfg(feature = "debug")]
    assert_byte_stream_empty(byte_stream, resource_file);

    Ok(data)
}

pub trait GroundTileExt {
    fn get_lowest_point(&self) -> f32;
}

impl GroundTileExt for GroundTile {
    fn get_lowest_point(&self) -> f32 {
        [
            self.lower_right_height,
            self.lower_left_height,
            self.upper_left_height,
            self.lower_right_height,
        ]
        .into_iter()
        .reduce(f32::max)
        .unwrap()
    }
}
