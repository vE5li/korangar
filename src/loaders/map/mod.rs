mod data;
mod resource;
mod vertices;

use std::collections::HashMap;
use std::sync::Arc;

use cgmath::Vector3;
use derive_new::new;

#[cfg(feature = "debug")]
pub use self::data::MapData;
use self::data::*;
pub use self::resource::{LightSettings, WaterSettings};
use self::vertices::{generate_tile_vertices, get_vertex_buffer, ground_water_vertices, load_textures, optional_vertex_buffer};
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{MemoryAllocator, NativeModelVertex};
use crate::loaders::{ByteConvertable, ByteStream, GameFileLoader, ModelLoader, TextureLoader};
use crate::world::*;

const MAP_OFFSET: f32 = 5.0;

#[derive(new)]
pub struct MapLoader {
    memory_allocator: Arc<MemoryAllocator>,
    #[new(default)]
    cache: HashMap<String, Arc<Map>>,
}

impl MapLoader {
    pub fn get(
        &mut self,
        resource_file: String,
        game_file_loader: &mut GameFileLoader,
        model_loader: &mut ModelLoader,
        texture_loader: &mut TextureLoader,
    ) -> Result<Arc<Map>, String> {
        match self.cache.get(&resource_file) {
            Some(map) => Ok(map.clone()),
            None => self.load(resource_file, game_file_loader, model_loader, texture_loader),
        }
    }

    fn load(
        &mut self,
        resource_file: String,
        game_file_loader: &mut GameFileLoader,
        model_loader: &mut ModelLoader,
        texture_loader: &mut TextureLoader,
    ) -> Result<Arc<Map>, String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load map from {}", &resource_file));

        let mut map_data = parse_map_data(&resource_file, game_file_loader)?;

        #[cfg(feature = "debug")]
        let map_data_clone = map_data.clone();

        let ground_data = parse_ground_data(map_data.ground_file.as_str(), game_file_loader)?;
        let mut gat_data = parse_gat_data(map_data.gat_file.as_str(), game_file_loader)?;

        let (tile_vertices, tile_picker_vertices) = generate_tile_vertices(&mut gat_data);
        let water_level = -map_data.water_settings.water_level.unwrap();
        let (ground_vertices, water_vertices) = ground_water_vertices(&ground_data, water_level);

        let ground_vertices = NativeModelVertex::to_vertices(ground_vertices);
        let ground_vertex_buffer = get_vertex_buffer(&self.memory_allocator, ground_vertices);
        let water_vertex_buffer = optional_vertex_buffer(&self.memory_allocator, water_vertices);
        let tile_vertex_buffer = optional_vertex_buffer(&self.memory_allocator, tile_vertices);
        let tile_picker_vertex_buffer = optional_vertex_buffer(&self.memory_allocator, tile_picker_vertices);

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
                let model = model_loader.get(game_file_loader, texture_loader, object_data.model_name.as_str(), reverse_order);

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

    resources.objects.iter_mut().for_each(|object| object.offset(offset));
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

fn parse_map_data(resource_file: &str, game_file_loader: &mut GameFileLoader) -> Result<MapData, String> {
    let bytes = game_file_loader.get(&format!("data\\{}.rsw", &resource_file))?;
    let mut byte_stream = ByteStream::new(&bytes);

    if <[u8; 4]>::from_bytes(&mut byte_stream, None) != [b'G', b'R', b'S', b'W'] {
        return Err(format!("failed to read magic number from {}.rsw", &resource_file));
    }

    let map_data = MapData::from_bytes(&mut byte_stream, None);

    #[cfg(feature = "debug")]
    byte_stream.assert_empty(resource_file);

    Ok(map_data)
}

fn parse_ground_data(ground_file: &str, game_file_loader: &mut GameFileLoader) -> Result<GroundData, String> {
    let bytes = game_file_loader.get(&format!("data\\{}", &ground_file))?;
    let mut byte_stream = ByteStream::new(&bytes);

    if <[u8; 4]>::from_bytes(&mut byte_stream, None) != [b'G', b'R', b'G', b'N'] {
        return Err(format!("failed to read magic number from {}", &ground_file));
    }

    let ground_data = GroundData::from_bytes(&mut byte_stream, None);

    #[cfg(feature = "debug")]
    byte_stream.assert_empty(ground_file);

    Ok(ground_data)
}

fn parse_gat_data(gat_file: &str, game_file_loader: &mut GameFileLoader) -> Result<GatData, String> {
    let bytes = game_file_loader.get(&format!("data\\{}", &gat_file))?;
    let mut byte_stream = ByteStream::new(&bytes);

    if <[u8; 4]>::from_bytes(&mut byte_stream, None) != [b'G', b'R', b'A', b'T'] {
        return Err(format!("failed to read magic number from {}", &gat_file));
    }

    let gat_data = GatData::from_bytes(&mut byte_stream, None);

    #[cfg(feature = "debug")]
    byte_stream.assert_empty(gat_file);

    Ok(gat_data)
}
