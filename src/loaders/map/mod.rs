mod resource;

use derive_new::new;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use std::collections::HashMap;
use cgmath::{ Vector3, Vector2, Deg };
use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::device::Device;
use vulkano::sync::{ GpuFuture, now };

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::types::ByteStream;
use crate::types::map::{ Map, Tile, TileType, WaterSettings, LightSettings, Object, LightSource, SoundSource, EffectSource };
use crate::graphics::{ Color, ModelVertex, Transform, NativeModelVertex, WaterVertex, TileVertex };
use crate::loaders::{ ModelLoader, TextureLoader, GameFileLoader };

use self::resource::ResourceType;

const MAP_OFFSET: f32 = 5.0;
const TILE_SIZE: f32 = 10.0;

#[derive(Copy, Clone, Debug)]
pub enum SurfaceType {
    Front,
    Right,
    Top
}

pub struct Surface {
    u: [f32; 4],
    v: [f32; 4],
    texture_index: i32,
    _light_map_index: i32,
    _color: Color,
}

impl Surface {

    pub fn new(u: [f32; 4], v: [f32; 4], texture_index: i32, light_map_index: i32, color: Color) -> Self {
        Self {
            u,
            v,
            texture_index: texture_index % 14, // TODO: remove % 14 and derive new
            _light_map_index: light_map_index,
            _color: color
        }
    }
}

#[derive(new)]
pub struct GroundTile {
    pub upper_left_height: f32,
    pub upper_right_height: f32,
    pub lower_left_height: f32,
    pub lower_right_height: f32,
    pub top_surface_index: i32,
    pub front_surface_index: i32,
    pub right_surface_index: i32,
}

impl GroundTile {
    
    pub fn get_lowest_point(&self) -> f32 {
        f32::max(self.lower_right_height, f32::max(self.lower_left_height, f32::max(self.upper_left_height, self.upper_right_height)))
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Heights {
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight
}

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
    game_file_loader: Rc<RefCell<GameFileLoader>>,
    device: Arc<Device>,
    #[new(default)]
    cache: HashMap<String, Arc<Map>>,
}

impl MapLoader {

    fn load(&mut self, model_loader: &mut ModelLoader, texture_loader: &mut TextureLoader, resource_file: &str) -> Result<Arc<Map>, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load map from {}", resource_file));

        let mut texture_future = now(self.device.clone()).boxed();

        let bytes = self.game_file_loader.borrow_mut().get(&format!("data\\{}", resource_file))?;
        let mut byte_stream = ByteStream::new(&bytes);

        let magic = byte_stream.string(4);
        
        if &magic != "GRSW" {
            return Err(format!("failed to read magic number from {}", resource_file));
        }

        let resource_version = byte_stream.version();
 
        if !resource_version.equals_or_above(1, 2) {
            return Err(format!("invalid resource version {}", resource_version));
        }

        // INI file
        byte_stream.skip(40);

        let ground_file = byte_stream.string(40);

        let gat_file = match resource_version.equals_or_above(1, 4) {
            true => Some(byte_stream.string(40)),
            false => None,
        };

        // SRC file
        byte_stream.skip(40);

        let mut water_settings = WaterSettings::new();

        if resource_version.equals_or_above(1, 3) {
            let water_level = byte_stream.float32();
            water_settings.water_level = -water_level;
        }

        if resource_version.equals_or_above(1, 8) {

            let water_type = byte_stream.integer32();
            let wave_height = byte_stream.float32();
            let wave_speed = byte_stream.float32();
            let wave_pitch = byte_stream.float32();

            water_settings.water_type = water_type as usize;
            water_settings.wave_height = wave_height;
            water_settings.wave_speed = wave_speed;
            water_settings.wave_pitch = wave_pitch;
        }

        if resource_version.equals_or_above(1, 9) {
            let water_animation_speed = byte_stream.integer32();
            water_settings.water_animation_speed = water_animation_speed as usize;
        }

        let mut light_settings = LightSettings::new();

        if resource_version.equals_or_above(1, 5) {

            let light_longitude = byte_stream.integer32();
            let light_latitude = byte_stream.integer32();
            let diffuse_color = byte_stream.color();
            let ambient_color = byte_stream.color();

            light_settings.light_longitude = light_longitude as isize;
            light_settings.light_latitude = light_latitude as isize;
            light_settings.diffuse_color = diffuse_color;
            light_settings.ambient_color = ambient_color;
        }

        if resource_version.equals_or_above(1, 7) {
            let _unknown = byte_stream.float32();
        }

        if resource_version.equals_or_above(1, 6) {

            let _ground_top = byte_stream.integer32();
            let _ground_bottom = byte_stream.integer32();
            let _ground_left = byte_stream.integer32();
            let _ground_right = byte_stream.integer32();
        }

        let object_count = byte_stream.integer32() as usize;

        let mut objects = Vec::new();
        let mut light_sources = Vec::new();
        let mut sound_sources = Vec::new();
        let mut effect_sources = Vec::new();

        for _index in 0..object_count {
            let type_index = byte_stream.integer32();
            let resource_type = ResourceType::from(type_index);

            match resource_type {

                ResourceType::Object => {

                    if resource_version.equals_or_above(1, 6) {

                        let name = byte_stream.string(40);
                        let _animation_type = byte_stream.integer32();
                        let _animation_speed = byte_stream.float32();
                        let _block_type = byte_stream.integer32();
                        let model_name = byte_stream.string(80);
                        let _node_name = byte_stream.string(80);
                        let position = byte_stream.vector3_flipped();
                        let rotation = byte_stream.vector3();
                        let scale = byte_stream.vector3();

                        let model = model_loader.get(texture_loader, &model_name, &mut texture_future)?;
                        let transform = Transform::from(position, rotation.map(Deg), scale);
                        let object = Object::new(Some(name), model_name, model, transform);
                        objects.push(object);
                    } else {

                        let model_name = byte_stream.string(80);
                        let _node_name = byte_stream.string(80);
                        let position = byte_stream.vector3_flipped();
                        let rotation = byte_stream.vector3();
                        let scale = byte_stream.vector3();

                        let model = model_loader.get(texture_loader, &model_name, &mut texture_future)?;
                        let transform = Transform::from(position, rotation.map(Deg), scale);
                        let object = Object::new(None, model_name, model, transform);
                        objects.push(object);
                    }
                },

                ResourceType::LightSource => {

                    let name = byte_stream.string(80);
                    let position = byte_stream.vector3_flipped();
                    let red = byte_stream.integer32() as u8;
                    let green = byte_stream.integer32() as u8;
                    let blue = byte_stream.integer32() as u8;
                    let color = Color::rgb(red, green, blue);
                    let range = byte_stream.float32();

                    light_sources.push(LightSource::new(name, position, color, range));
                },

                ResourceType::SoundSource => {

                    let name = byte_stream.string(80);
                    let sound_file = byte_stream.string(80);
                    let position = byte_stream.vector3_flipped();
                    let volume = byte_stream.float32();
                    let width = byte_stream.integer32();
                    let height = byte_stream.integer32();
                    let range = byte_stream.float32();

                    let cycle = match resource_version.equals_or_above(2, 0) {
                        true => byte_stream.float32(),
                        false => 4.0,
                    };

                    sound_sources.push(SoundSource::new(name, sound_file, position, volume, width as usize, height as usize, range, cycle));
                },

                ResourceType::EffectSource => {

                    let name = byte_stream.string(80);
                    let position = byte_stream.vector3_flipped();
                    let effect_type = byte_stream.integer32();
                    let emit_speed = byte_stream.float32();

                    let _param0 = byte_stream.float32();
                    let _param1 = byte_stream.float32();
                    let _param2 = byte_stream.float32();
                    let _param3 = byte_stream.float32();

                    effect_sources.push(EffectSource::new(name, position, effect_type as usize, emit_speed));
                },
            }
        }

        // TODO;

        #[cfg(feature = "debug")]
        byte_stream.assert_empty(resource_file);

        let bytes = self.game_file_loader.borrow_mut().get(&format!("data\\{}", ground_file))?;
        let mut byte_stream = ByteStream::new(&bytes);

        let magic = byte_stream.string(4);
        if &magic != "GRGN" {
            return Err(format!("failed to read magic number from {}", ground_file));
        }

        let ground_version = byte_stream.version();

        if !ground_version.equals_or_above(1, 6) {
            return Err(format!("invalid ground version {}", ground_version));
        }

        let width = byte_stream.integer32() as usize;
        let height = byte_stream.integer32() as usize;
        let _zoom = byte_stream.float32();
        let texture_count = byte_stream.integer32();
        let texture_name_length = byte_stream.integer32();

        let mut textures = Vec::new();

        for _index in 0..texture_count {
            let texture_name = byte_stream.string(texture_name_length as usize);
            let texture = texture_loader.get(&texture_name, &mut texture_future)?;
            textures.push(texture);
        }

        let light_map_count = byte_stream.integer32() as usize;
        let light_map_width = byte_stream.integer32() as usize;
        let light_map_height = byte_stream.integer32() as usize;
        let _light_map_cells_per_grid = byte_stream.integer32();

        let dimensions = width * height;
        let light_map_dimensions = light_map_width * light_map_height;

        match ground_version.equals_or_above(1, 7) {
            true => byte_stream.skip(light_map_count * light_map_dimensions * 4),
            false => byte_stream.skip(light_map_count * 16),
        }

        let surface_count = byte_stream.integer32();
        let mut surfaces = Vec::new();

        for _index in 0..surface_count {

            let u = [byte_stream.float32(), byte_stream.float32(), byte_stream.float32(), byte_stream.float32()];
            let v = [byte_stream.float32(), byte_stream.float32(), byte_stream.float32(), byte_stream.float32()];

            let texture_index = byte_stream.integer16() as i32;
            let light_map_index = byte_stream.integer16() as i32;
            let color_bgra = byte_stream.slice(4);

            let color = Color::rgb(color_bgra[2], color_bgra[1], color_bgra[0]);
            surfaces.push(Surface::new(u, v, texture_index, light_map_index, color));
        }

        let mut ground_tiles = Vec::new();

        for _index in 0..dimensions {

            let upper_left_height = byte_stream.float32();
            let upper_right_height = byte_stream.float32();
            let lower_left_height = byte_stream.float32();
            let lower_right_height = byte_stream.float32();

            let top_surface_index = match ground_version.equals_or_above(1, 7) {
                true => byte_stream.integer32(),
                false => byte_stream.integer16() as i32,
            };

            let front_surface_index = match ground_version.equals_or_above(1, 7) {
                true => byte_stream.integer32(),
                false => byte_stream.integer16() as i32,
            };

            let right_surface_index = match ground_version.equals_or_above(1, 7) {
                true => byte_stream.integer32(),
                false => byte_stream.integer16() as i32,
            };

            ground_tiles.push(GroundTile::new(upper_left_height, upper_right_height, lower_left_height, lower_right_height, top_surface_index, front_surface_index, right_surface_index));
        }

        #[cfg(feature = "debug")]
        byte_stream.assert_empty(&ground_file);

        let mut map_width = width;
        let mut map_height = height;
        let mut tiles = Vec::new();
        let mut tile_vertex_buffer = None;
        let mut tile_picker_vertex_buffer = None;

        if let Some(gat_file) = gat_file {

            let bytes = self.game_file_loader.borrow_mut().get(&format!("data\\{}", gat_file))?;
            let mut byte_stream = ByteStream::new(&bytes);

            let magic = byte_stream.string(4);
            if &magic != "GRAT" {
                return Err(format!("failed to read magic number from {}", gat_file));
            }

            let gat_version = byte_stream.version();

            if !gat_version.equals(1, 2) {
                return Err(format!("invalid gat version {}", gat_version));
            }

            map_width = byte_stream.integer32() as usize; // todo: unsigned
            map_height = byte_stream.integer32() as usize; // todo: unsigned

            let mut tile_vertices = Vec::new();
            let mut tile_picker_vertices = Vec::new();

            for y in 0..map_height {
                for x in 0..map_width {

                    let upper_left_height = -byte_stream.float32();
                    let upper_right_height = -byte_stream.float32();
                    let lower_left_height = -byte_stream.float32();
                    let lower_right_height = -byte_stream.float32();
                    let tile_type_index = byte_stream.byte();
                    let tile_type = TileType::new(tile_type_index);

                    // unknown
                    byte_stream.skip(3);

                    tiles.push(Tile::new(upper_left_height, upper_right_height, lower_left_height, lower_right_height, tile_type));

                    if tile_type.is_none() {
                        continue;
                    }

                    let offset = Vector2::new(x as f32 * 5.0, y as f32 * 5.0);

                    let first_position = Vector3::new(offset.x, upper_left_height + 1.0, offset.y);
                    let second_position = Vector3::new(offset.x + 5.0, upper_right_height + 1.0, offset.y);
                    let third_position = Vector3::new(offset.x + 5.0, lower_right_height + 1.0, offset.y + 5.0);
                    let fourth_position = Vector3::new(offset.x, lower_left_height + 1.0, offset.y + 5.0);

                    let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
                    let second_normal = NativeModelVertex::calculate_normal(fourth_position, first_position, third_position);

                    let first_texture_coordinates = Vector2::new(0.0, 0.0);
                    let second_texture_coordinates = Vector2::new(0.0, 1.0);
                    let third_texture_coordinates = Vector2::new(1.0, 1.0);
                    let fourth_texture_coordinates = Vector2::new(1.0, 0.0);

                    tile_vertices.push(ModelVertex::new(first_position, first_normal, first_texture_coordinates, tile_type_index as i32));
                    tile_vertices.push(ModelVertex::new(second_position, first_normal, second_texture_coordinates, tile_type_index as i32));
                    tile_vertices.push(ModelVertex::new(third_position, first_normal, third_texture_coordinates, tile_type_index as i32));

                    tile_vertices.push(ModelVertex::new(first_position, second_normal, first_texture_coordinates, tile_type_index as i32));
                    tile_vertices.push(ModelVertex::new(third_position, second_normal, third_texture_coordinates, tile_type_index as i32));
                    tile_vertices.push(ModelVertex::new(fourth_position, second_normal, fourth_texture_coordinates, tile_type_index as i32));

                    let color = (x as u32 + 1) | ((y as u32 + 1) << 16);
                    tile_picker_vertices.push(TileVertex::new(first_position, color));
                    tile_picker_vertices.push(TileVertex::new(second_position, color));
                    tile_picker_vertices.push(TileVertex::new(third_position, color));

                    tile_picker_vertices.push(TileVertex::new(first_position, color));
                    tile_picker_vertices.push(TileVertex::new(third_position, color));
                    tile_picker_vertices.push(TileVertex::new(fourth_position, color));
                }
            }

            #[cfg(feature = "debug")]
            byte_stream.assert_empty(&gat_file);

            let vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, tile_vertices.into_iter()).unwrap();
            tile_vertex_buffer = Some(vertex_buffer);

            let vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, tile_picker_vertices.into_iter()).unwrap();
            tile_picker_vertex_buffer = Some(vertex_buffer);
        }

        let mut native_ground_vertices = Vec::new();
        let mut water_vertices = Vec::new();

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
                        let neighbor_tile = &ground_tiles[neighbor_x + neighbor_y * width];

                        let (surface_offset, surface_height) = surface_alignment[0];
                        let height = get_tile_height_at(current_tile, surface_height);
                        let first_position = Vector3::new((x + surface_offset.x) as f32 * TILE_SIZE, -height, (y + surface_offset.y) as f32 * TILE_SIZE);

                        let (surface_offset, surface_height) = surface_alignment[1];
                        let height = get_tile_height_at(current_tile, surface_height);
                        let second_position = Vector3::new((x + surface_offset.x) as f32 * TILE_SIZE, -height, (y + surface_offset.y) as f32 * TILE_SIZE);

                        let (surface_offset, surface_height) = surface_alignment[2];
                        let height = get_tile_height_at(neighbor_tile, surface_height);
                        let third_position = Vector3::new((x + surface_offset.x) as f32 * TILE_SIZE, -height, (y + surface_offset.y) as f32 * TILE_SIZE);

                        let (surface_offset, surface_height) = surface_alignment[3];
                        let height = get_tile_height_at(neighbor_tile, surface_height);
                        let fourth_position = Vector3::new((x + surface_offset.x) as f32 * TILE_SIZE, -height, (y + surface_offset.y) as f32 * TILE_SIZE);

                        let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
                        let second_normal = NativeModelVertex::calculate_normal(fourth_position, first_position, third_position);

                        let ground_surface = &surfaces[surface_index as usize];

                        let first_texture_coordinates = Vector2::new(ground_surface.u[0], ground_surface.v[0]);
                        let second_texture_coordinates = Vector2::new(ground_surface.u[1], ground_surface.v[1]);
                        let third_texture_coordinates = Vector2::new(ground_surface.u[3], ground_surface.v[3]);
                        let fourth_texture_coordinates = Vector2::new(ground_surface.u[2], ground_surface.v[2]);

                        native_ground_vertices.push(NativeModelVertex::new(first_position, first_normal, first_texture_coordinates, ground_surface.texture_index));
                        native_ground_vertices.push(NativeModelVertex::new(second_position, first_normal, second_texture_coordinates, ground_surface.texture_index));
                        native_ground_vertices.push(NativeModelVertex::new(third_position, first_normal, third_texture_coordinates, ground_surface.texture_index));

                        native_ground_vertices.push(NativeModelVertex::new(first_position, second_normal, first_texture_coordinates, ground_surface.texture_index));
                        native_ground_vertices.push(NativeModelVertex::new(third_position, second_normal, third_texture_coordinates, ground_surface.texture_index));
                        native_ground_vertices.push(NativeModelVertex::new(fourth_position, second_normal, fourth_texture_coordinates, ground_surface.texture_index));
                    }
                }

                if -current_tile.get_lowest_point() < water_settings.water_level {

                    let first_position = Vector3::new(x as f32 * TILE_SIZE, water_settings.water_level, y as f32 * TILE_SIZE);
                    let second_position = Vector3::new(TILE_SIZE + x as f32 * TILE_SIZE, water_settings.water_level, y as f32 * TILE_SIZE);
                    let third_position = Vector3::new(TILE_SIZE + x as f32 * TILE_SIZE, water_settings.water_level, TILE_SIZE + y as f32 * TILE_SIZE);
                    let fourth_position = Vector3::new(x as f32 * TILE_SIZE, water_settings.water_level, TILE_SIZE + y as f32 * TILE_SIZE);

                    water_vertices.push(WaterVertex::new(first_position));
                    water_vertices.push(WaterVertex::new(second_position));
                    water_vertices.push(WaterVertex::new(third_position));

                    water_vertices.push(WaterVertex::new(first_position));
                    water_vertices.push(WaterVertex::new(third_position));
                    water_vertices.push(WaterVertex::new(fourth_position));
                }
            }
        }

        /*let row_size = width * 6;

        for index in 0..native_ground_vertices.len() / 6 {

            let base_index = index * 6;
            let mut indices = vec![base_index + 5];

            if base_index + 6 < native_ground_vertices.len() {
                indices.push(base_index + 6);
                indices.push(base_index + 9);
            }

            if base_index > row_size {
                if base_index >= row_size + 10 {
                    indices.push(base_index - row_size - 10);
                    indices.push(base_index - row_size - 8);
                }

                indices.push(base_index - row_size - 5);
            }

            let new_normal = indices.iter()
                .map(|index| native_ground_vertices[*index].normal)
                .fold(Vector3::new(0.0, 0.0, 0.0), |sum, normal| sum + normal);

            indices.iter().for_each(|index| native_ground_vertices[*index].normal = new_normal);
        }*/

        let ground_vertices = NativeModelVertex::to_vertices(native_ground_vertices);
        let ground_vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, ground_vertices.into_iter()).unwrap();

        let water_vertex_buffer = match !water_vertices.is_empty() {
            true => CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, water_vertices.into_iter()).unwrap().into(),
            false => None,
        };

        let offset = Vector3::new(width as f32 * MAP_OFFSET, 0.0, height as f32 * MAP_OFFSET);
        objects.iter_mut().for_each(|object| object.offset(offset));
        light_sources.iter_mut().for_each(|light_source| light_source.offset(offset));
        sound_sources.iter_mut().for_each(|sound_source| sound_source.offset(offset));
        effect_sources.iter_mut().for_each(|effect_source| effect_source.offset(offset));

        let map = Arc::new(Map::new(resource_version, ground_version, map_width, map_height, water_settings, light_settings, tiles, ground_vertex_buffer, water_vertex_buffer, textures, objects, light_sources, sound_sources, effect_sources, tile_picker_vertex_buffer.unwrap(), tile_vertex_buffer.unwrap()));

        self.cache.insert(resource_file.to_string(), map.clone());

        texture_future.flush().unwrap();
        texture_future.cleanup_finished();

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(map)
    }

    pub fn get(&mut self, model_loader: &mut ModelLoader, texture_loader: &mut TextureLoader, resource_file: &str) -> Result<Arc<Map>, String> {
        match self.cache.get(resource_file) {
            Some(map) => Ok(map.clone()),
            None => self.load(model_loader, texture_loader, resource_file),
        }
    }
}
