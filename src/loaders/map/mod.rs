pub mod map_data;
pub mod resource;

use std::collections::HashMap;
use std::slice::IterMut;
use std::sync::Arc;

use cgmath::{Deg, Vector2, Vector3};
use derive_new::new;
use procedural::*;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};

use self::map_data::*;
use self::resource::ResourceType;
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{Color, MemoryAllocator, ModelVertex, NativeModelVertex, PickerTarget, Texture, TileVertex, Transform, WaterVertex};
use crate::loaders::{ByteConvertable, ByteStream, GameFileLoader, ModelLoader, TextureLoader, Version};
use crate::world::*;

const MAP_OFFSET: f32 = 5.0;
const TILE_SIZE: f32 = 10.0;

pub fn tile_surface_index(tile: &GroundTile, surface_type: SurfaceType) -> i32 {
    match surface_type {
        SurfaceType::Front => tile.front_surface_index,
        SurfaceType::Right => tile.right_surface_index,
        SurfaceType::Top => tile.top_surface_index,
    }
}

pub fn get_tile_height_at(tile: &GroundTile, point: Heights) -> f32 {
    match point {
        Heights::UpperLeft => tile.upper_left_height,
        Heights::UpperRight => tile.upper_right_height,
        Heights::LowerLeft => tile.lower_left_height,
        Heights::LowerRight => tile.lower_right_height,
    }
}

pub fn tile_surface_alignment(surface_type: SurfaceType) -> [(Vector2<usize>, Heights); 4] {
    match surface_type {
        SurfaceType::Front => [
            (Vector2::new(0, 1), Heights::LowerLeft),
            (Vector2::new(1, 1), Heights::LowerRight),
            (Vector2::new(1, 1), Heights::UpperRight),
            (Vector2::new(0, 1), Heights::UpperLeft),
        ],
        SurfaceType::Right => [
            (Vector2::new(1, 1), Heights::LowerRight),
            (Vector2::new(1, 0), Heights::UpperRight),
            (Vector2::new(1, 0), Heights::UpperLeft),
            (Vector2::new(1, 1), Heights::LowerLeft),
        ],
        SurfaceType::Top => [
            (Vector2::new(0, 0), Heights::UpperLeft),
            (Vector2::new(1, 0), Heights::UpperRight),
            (Vector2::new(1, 1), Heights::LowerRight),
            (Vector2::new(0, 1), Heights::LowerLeft),
        ],
    }
}

pub fn neighbor_tile_index(surface_type: SurfaceType) -> Vector2<usize> {
    match surface_type {
        SurfaceType::Front => Vector2::new(0, 1),
        SurfaceType::Right => Vector2::new(1, 0),
        SurfaceType::Top => Vector2::new(0, 0),
    }
}

#[derive(new)]
pub struct MapLoader {
    memory_allocator: Arc<MemoryAllocator>,
    #[new(default)]
    cache: HashMap<String, Arc<Map>>,
}

impl MapLoader {
    fn load(
        &mut self,
        resource_file: String,
        game_file_loader: &mut GameFileLoader,
        model_loader: &mut ModelLoader,
        texture_loader: &mut TextureLoader,
    ) -> Result<Arc<Map>, String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load map from {}", &resource_file));

        let mut map_data = parse_map_data(&resource_file, model_loader, game_file_loader, texture_loader)?;

        //replace with attribute
        let water_settings = &mut map_data.water_settings;
        water_settings.water_level = Some(-water_settings.water_level.unwrap());
        let water_level = water_settings.water_level.unwrap();

        let ground_data = parse_ground_data(map_data.ground_file.as_str(), game_file_loader, texture_loader)?;

        let mut gat_data = parse_gat_data(map_data.gat_file.unwrap().as_str(), game_file_loader)?;

        let (tile_vertices, tile_picker_vertices) = generate_vertices(&mut gat_data);
        let (tile_vertex_buffer, tile_picker_vertex_buffer, native_ground_vertices, water_vertices) = self.generate_vertices(tile_vertices, tile_picker_vertices, &ground_data, water_level);
        let ground_vertices = NativeModelVertex::to_vertices(native_ground_vertices);
        let ground_vertex_buffer = self.generate_ground_vertex_buffer(ground_vertices);
        let water_vertex_buffer = self.generate_water_vertex_buffer(water_vertices);

        apply_map_offset(&ground_data, &mut map_data.resources);

        let textures = load_textures(&ground_data, texture_loader, game_file_loader);


        let map = Arc::new(Map::new(
            map_data.version,
            ground_data.version,
            gat_data.map_width as usize,
            gat_data.map_height as usize,
            map_data.water_settings,
            map_data.light_settings,
            gat_data.tiles,
            ground_vertex_buffer,
            water_vertex_buffer,
            textures,
            map_data.resources.objects,
            map_data.resources.light_sources,
            map_data.resources.sound_sources,
            map_data.resources.effect_sources,
            tile_picker_vertex_buffer.unwrap(),
            tile_vertex_buffer.unwrap(),
        ));

        self.cache.insert(resource_file, map.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(map)
    }

    fn generate_water_vertex_buffer(&mut self, water_vertices: Vec<WaterVertex>) -> Option<Arc<CpuAccessibleBuffer<[WaterVertex]>>> {
        let water_vertex_buffer = match !water_vertices.is_empty() {
            true => CpuAccessibleBuffer::from_iter(
                &*self.memory_allocator,
                BufferUsage {
                    vertex_buffer: true,
                    ..Default::default()
                },
                false,
                water_vertices.into_iter(),
            )
            .unwrap()
            .into(),
            false => None,
        };
        water_vertex_buffer
    }

    fn generate_ground_vertex_buffer(&mut self, ground_vertices: Vec<ModelVertex>) -> Arc<CpuAccessibleBuffer<[ModelVertex]>> {
        let ground_vertex_buffer = CpuAccessibleBuffer::from_iter(
            &*self.memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..Default::default()
            },
            false,
            ground_vertices.into_iter(),
        )
        .unwrap();
        ground_vertex_buffer
    }

    fn generate_vertices(&mut self, tile_vertices: Vec<ModelVertex>, tile_picker_vertices: Vec<TileVertex>, ground_data: &GroundData, water_level: f32)
     -> (Option<Arc<CpuAccessibleBuffer<[ModelVertex]>>>, Option<Arc<CpuAccessibleBuffer<[TileVertex]>>>, Vec<NativeModelVertex>, Vec<WaterVertex>) {
        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            &*self.memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..Default::default()
            },
            false,
            tile_vertices.into_iter(),
        )
        .unwrap();
        let tile_vertex_buffer = Some(vertex_buffer);
        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            &*self.memory_allocator,
            BufferUsage {
                vertex_buffer: true,
                ..Default::default()
            },
            false,
            tile_picker_vertices.into_iter(),
        )
        .unwrap();
        //vertices calculation
        let tile_picker_vertex_buffer = Some(vertex_buffer);
        let mut native_ground_vertices = Vec::new();
        let mut water_vertices = Vec::new();
        //ground_data.ground_tiles.iter().for_each(|ground_tile| {
        let width = ground_data.width as usize;
        let height = ground_data.height as usize;
        let ground_tiles = &ground_data.ground_tiles;
        for x in 0..width {
            for y in 0..height {
                let current_tile = &ground_tiles[x + y * width];

                for surface_type in [SurfaceType::Front, SurfaceType::Right, SurfaceType::Top].iter() {
                    let surface_index = tile_surface_index(current_tile, *surface_type);

                    if surface_index > -1 {
                        let surface_alignment = tile_surface_alignment(*surface_type);
                        let neighbor_tile_index = neighbor_tile_index(*surface_type);

                        let neighbor_x = x + neighbor_tile_index.x;
                        let neighbor_y = y + neighbor_tile_index.y;
                        let Some(neighbor_tile) = ground_tiles.get(neighbor_x + neighbor_y * width) else {
                            continue;
                        };

                        let (surface_offset, surface_height) = surface_alignment[0];
                        let height = get_tile_height_at(current_tile, surface_height);
                        let first_position = Vector3::new(
                            (x + surface_offset.x) as f32 * TILE_SIZE,
                            -height,
                            (y + surface_offset.y) as f32 * TILE_SIZE,
                        );

                        let (surface_offset, surface_height) = surface_alignment[1];
                        let height = get_tile_height_at(current_tile, surface_height);
                        let second_position = Vector3::new(
                            (x + surface_offset.x) as f32 * TILE_SIZE,
                            -height,
                            (y + surface_offset.y) as f32 * TILE_SIZE,
                        );

                        let (surface_offset, surface_height) = surface_alignment[2];
                        let height = get_tile_height_at(neighbor_tile, surface_height);
                        let third_position = Vector3::new(
                            (x + surface_offset.x) as f32 * TILE_SIZE,
                            -height,
                            (y + surface_offset.y) as f32 * TILE_SIZE,
                        );

                        let (surface_offset, surface_height) = surface_alignment[3];
                        let height = get_tile_height_at(neighbor_tile, surface_height);
                        let fourth_position = Vector3::new(
                            (x + surface_offset.x) as f32 * TILE_SIZE,
                            -height,
                            (y + surface_offset.y) as f32 * TILE_SIZE,
                        );

                        let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
                        let second_normal = NativeModelVertex::calculate_normal(fourth_position, first_position, third_position);

                        let ground_surface = &ground_data.surfaces[surface_index as usize];

                        let first_texture_coordinates = Vector2::new(ground_surface.u[0], ground_surface.v[0]);
                        let second_texture_coordinates = Vector2::new(ground_surface.u[1], ground_surface.v[1]);
                        let third_texture_coordinates = Vector2::new(ground_surface.u[3], ground_surface.v[3]);
                        let fourth_texture_coordinates = Vector2::new(ground_surface.u[2], ground_surface.v[2]);

                        native_ground_vertices.push(NativeModelVertex::new(
                            first_position,
                            first_normal,
                            first_texture_coordinates,
                            ground_surface.texture_index as i32,
                            0.0,
                        ));
                        native_ground_vertices.push(NativeModelVertex::new(
                            second_position,
                            first_normal,
                            second_texture_coordinates,
                            ground_surface.texture_index as i32,
                            0.0,
                        ));
                        native_ground_vertices.push(NativeModelVertex::new(
                            third_position,
                            first_normal,
                            third_texture_coordinates,
                            ground_surface.texture_index as i32,
                            0.0,
                        ));

                        native_ground_vertices.push(NativeModelVertex::new(
                            first_position,
                            second_normal,
                            first_texture_coordinates,
                            ground_surface.texture_index as i32,
                            0.0,
                        ));
                        native_ground_vertices.push(NativeModelVertex::new(
                            third_position,
                            second_normal,
                            third_texture_coordinates,
                            ground_surface.texture_index as i32,
                            0.0,
                        ));
                        native_ground_vertices.push(NativeModelVertex::new(
                            fourth_position,
                            second_normal,
                            fourth_texture_coordinates,
                            ground_surface.texture_index as i32,
                            0.0,
                        ));
                    }
                }

                if -current_tile.get_lowest_point() < water_level {
                    let first_position = Vector3::new(x as f32 * TILE_SIZE, water_level, y as f32 * TILE_SIZE);
                    let second_position = Vector3::new(TILE_SIZE + x as f32 * TILE_SIZE, water_level, y as f32 * TILE_SIZE);
                    let third_position = Vector3::new(TILE_SIZE + x as f32 * TILE_SIZE, water_level, TILE_SIZE + y as f32 * TILE_SIZE);
                    let fourth_position = Vector3::new(x as f32 * TILE_SIZE, water_level, TILE_SIZE + y as f32 * TILE_SIZE);

                    water_vertices.push(WaterVertex::new(first_position));
                    water_vertices.push(WaterVertex::new(second_position));
                    water_vertices.push(WaterVertex::new(third_position));

                    water_vertices.push(WaterVertex::new(first_position));
                    water_vertices.push(WaterVertex::new(third_position));
                    water_vertices.push(WaterVertex::new(fourth_position));
                }
            }
        }
        (tile_vertex_buffer, tile_picker_vertex_buffer, native_ground_vertices, water_vertices)
    }

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
}

fn load_textures(ground_data: &GroundData, texture_loader: &mut TextureLoader, game_file_loader: &mut GameFileLoader) -> Vec<Texture> {
    let mut textures = Vec::new();
    ground_data.textures.iter().for_each(|texture_name| {
        let texture = texture_loader.get(&texture_name, game_file_loader).unwrap();
        textures.push(texture);
    });
    textures
}

fn generate_vertices(gat_data: &mut GatData) -> (Vec<ModelVertex>, Vec<TileVertex>) {
    let mut tile_vertices = Vec::new();
    let mut tile_picker_vertices = Vec::new();
    //gat_data.tiles.iter_mut().for_each(|tile| {
    let mut count = 0;
    for y in 0..gat_data.map_height {
        for x in 0..gat_data.map_width {
            let mut tile = &mut gat_data.tiles[count];
            //replace with attribute
            tile.upper_left_height = -tile.upper_left_height;
            tile.upper_right_height = -tile.upper_right_height;
            tile.lower_left_height = -tile.lower_left_height;
            tile.lower_right_height = -tile.lower_right_height;
            count += 1;

            if tile.tile_type.is_none() {
                continue;
            }

            let offset = Vector2::new(x as f32 * 5.0, y as f32 * 5.0);

            let first_position = Vector3::new(offset.x, tile.upper_left_height + 1.0, offset.y);
            let second_position = Vector3::new(offset.x + 5.0, tile.upper_right_height + 1.0, offset.y);
            let third_position = Vector3::new(offset.x + 5.0, tile.lower_right_height + 1.0, offset.y + 5.0);
            let fourth_position = Vector3::new(offset.x, tile.lower_left_height + 1.0, offset.y + 5.0);

            let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
            let second_normal = NativeModelVertex::calculate_normal(fourth_position, first_position, third_position);

            let first_texture_coordinates = Vector2::new(0.0, 0.0);
            let second_texture_coordinates = Vector2::new(0.0, 1.0);
            let third_texture_coordinates = Vector2::new(1.0, 1.0);
            let fourth_texture_coordinates = Vector2::new(1.0, 0.0);

            let tile_type_index = tile.tile_type.0 as i32;

            tile_vertices.push(ModelVertex::new(
                first_position,
                first_normal,
                first_texture_coordinates,
                tile_type_index,
                0.0,
            ));
            tile_vertices.push(ModelVertex::new(
                second_position,
                first_normal,
                second_texture_coordinates,
                tile_type_index,
                0.0,
            ));
            tile_vertices.push(ModelVertex::new(
                third_position,
                first_normal,
                third_texture_coordinates,
                tile_type_index,
                0.0,
            ));

            tile_vertices.push(ModelVertex::new(
                first_position,
                second_normal,
                first_texture_coordinates,
                tile_type_index,
                0.0,
            ));
            tile_vertices.push(ModelVertex::new(
                third_position,
                second_normal,
                third_texture_coordinates,
                tile_type_index,
                0.0,
            ));
            tile_vertices.push(ModelVertex::new(
                fourth_position,
                second_normal,
                fourth_texture_coordinates,
                tile_type_index,
                0.0,
            ));

            let first_position = Vector3::new(offset.x, tile.upper_left_height, offset.y);
            let second_position = Vector3::new(offset.x + 5.0, tile.upper_right_height, offset.y);
            let third_position = Vector3::new(offset.x + 5.0, tile.lower_right_height, offset.y + 5.0);
            let fourth_position = Vector3::new(offset.x, tile.lower_left_height, offset.y + 5.0);

            let color = PickerTarget::Tile(x as u16, y as u16).into();
            tile_picker_vertices.push(TileVertex::new(first_position, color));
            tile_picker_vertices.push(TileVertex::new(second_position, color));
            tile_picker_vertices.push(TileVertex::new(third_position, color));

            tile_picker_vertices.push(TileVertex::new(first_position, color));
            tile_picker_vertices.push(TileVertex::new(third_position, color));
            tile_picker_vertices.push(TileVertex::new(fourth_position, color));
        }
    }
    (tile_vertices, tile_picker_vertices)
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

fn parse_map_data(
    resource_file: &str,
    model_loader: &mut ModelLoader,
    game_file_loader: &mut GameFileLoader,
    texture_loader: &mut TextureLoader,
) -> Result<MapData, String> {
    let bytes = game_file_loader.get(&format!("data\\{}.rsw", &resource_file))?;
    let mut byte_stream = ByteStream::new(&bytes);

    if byte_stream.string(4) != "GRSW" {
        return Err(format!("failed to read magic number from {}.rsw", &resource_file));
    }

    let mut map_data = MapData::from_bytes(&mut byte_stream, None);

    // Loading object models
    map_data.resources.objects.iter_mut().for_each(|object| {
        let array: [f32; 3] = object.transform.scale.into();
        let reverse_order = array.into_iter().fold(1.0, |a, b| a * b).is_sign_negative();
        let model = model_loader.get(game_file_loader, texture_loader, object.model_name.as_str(), reverse_order);
        object.set_model(model.unwrap());
    });

    #[cfg(feature = "debug")]
    byte_stream.assert_empty(&resource_file);
    Ok(map_data)
}

fn parse_ground_data(ground_file: &str, game_file_loader: &mut GameFileLoader, texture_loader: &mut TextureLoader) -> Result<GroundData, String> {
    let bytes = game_file_loader.get(&format!("data\\{}", &ground_file))?;
    let mut byte_stream = ByteStream::new(&bytes);
    let magic = byte_stream.string(4);
    if &magic != "GRGN" {
        return Err(format!("failed to read magic number from {}", &ground_file));
    }
    let ground_data = GroundData::from_bytes(&mut byte_stream, None);

    #[cfg(feature = "debug")]
    byte_stream.assert_empty(&ground_file);
    Ok(ground_data)
}

fn parse_gat_data(gat_file: &str, game_file_loader: &mut GameFileLoader) -> Result<GatData, String> {
    let bytes = game_file_loader.get(&format!("data\\{}", &gat_file))?;
    let mut byte_stream = ByteStream::new(&bytes);
    let magic = byte_stream.string(4);
    if &magic != "GRAT" {
        return Err(format!("failed to read magic number from {}", &gat_file));
    }

    let gat_data = GatData::from_bytes(&mut byte_stream, None);
    #[cfg(feature = "debug")]
    byte_stream.assert_empty(&gat_file);
    Ok(gat_data)
}
