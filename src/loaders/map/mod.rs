mod resource;

use derive_new::new;
use std::sync::Arc;
use std::collections::HashMap;
use std::fs::read;
use cgmath::{ Vector3, Vector2, Rad, Deg };
use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::device::Device;
use vulkano::sync::{ GpuFuture, now };

#[cfg(feature = "debug")]
use debug::*;
use map::{ Map, Tile, TileType, Object, LightSource, SoundSource, EffectSource };
use graphics::{ Color, ModelVertex, Transform, NativeModelVertex };
use loaders::{ ModelLoader, TextureLoader };

use super::ByteStream;

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
        return Self {
            u,
            v,
            texture_index: texture_index % 10, // TODO: remove % 10 and derive new
            _light_map_index: light_map_index,
            _color: color
        };
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

#[derive(Copy, Clone, Debug)]
pub enum Heights {
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight
}

pub fn tile_surface_index(tile: &GroundTile, surface_type: SurfaceType) -> i32 {
    match surface_type {
        SurfaceType::Front => return tile.front_surface_index,
        SurfaceType::Right => return tile.right_surface_index,
        SurfaceType::Top => return tile.top_surface_index,
    }
}

pub fn get_tile_height_at(tile: &GroundTile, point: Heights) -> f32 {
    match point {
        Heights::UpperLeft => return tile.upper_left_height,
        Heights::UpperRight => return tile.upper_right_height,
        Heights::LowerLeft => return tile.lower_left_height,
        Heights::LowerRight => return tile.lower_right_height,
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
        SurfaceType::Front => return Vector2::new(0, 1),
        SurfaceType::Right => return Vector2::new(1, 0),
        SurfaceType::Top => return Vector2::new(0, 0),
    }
}

#[derive(new)]
pub struct MapLoader {
    #[new(default)]
    cache: HashMap<String, Arc<Map>>,
    device: Arc<Device>,
}

impl MapLoader {

    fn load(&mut self, model_loader: &mut ModelLoader, texture_loader: &mut TextureLoader, resource_file: String) -> Arc<Map> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load map from {}{}{}", magenta(), resource_file, none()));

        let mut texture_future = now(self.device.clone()).boxed();

        let bytes = read(resource_file.clone()).expect("u r very stupid");
        let mut byte_stream = ByteStream::new(bytes.iter());

        let magic = byte_stream.string(4);
        assert!(&magic == "GRSW", "failed to read magic number");

        let version = byte_stream.version();

        if !version.equals_or_above(1, 2) {
            panic!("failed to read resource version");
        }

        #[cfg(feature = "debug_map")]
        print_debug!("version {}{}{}", magenta(), version, none());

        // INI file
        byte_stream.skip(40);

        let ground_file = byte_stream.string(40);

        #[cfg(feature = "debug_map")]
        print_debug!("ground file {}{}{}", magenta(), ground_file, none());

        let gat_file = match version.equals_or_above(1, 4) {
            true => Some(byte_stream.string(40)),
            false => None,
        };

        #[cfg(feature = "debug_map")]
        print_debug!("gat file {}{:?}{}", magenta(), gat_file, none());

        // SRC file
        byte_stream.skip(40);

        if version.equals_or_above(1, 3) {
            let _water_level = byte_stream.float32();

            #[cfg(feature = "debug_map")]
            print_debug!("water level {}{}{}", magenta(), _water_level, none());
        }

        if version.equals_or_above(1, 8) {

            let _water_type = byte_stream.integer32();
            let _wave_height = byte_stream.float32();
            let _wave_speed = byte_stream.float32();
            let _wave_pitch = byte_stream.float32();

            #[cfg(feature = "debug_map")]
            {
                print_debug!("water type {}{}{}", magenta(), _water_type, none());
                print_debug!("wave height {}{}{}", magenta(), _wave_height, none());
                print_debug!("wave speed {}{}{}", magenta(), _wave_speed, none());
                print_debug!("wave pitch {}{}{}", magenta(), _wave_pitch, none());
            }
        }

        if version.equals_or_above(1, 9) {
            let _water_animation_speed = byte_stream.integer32();

            #[cfg(feature = "debug_map")]
            print_debug!("water animation speed {}{}{}", magenta(), _water_animation_speed, none());
        }

        let mut ambient_light_color = Color::new(255, 255, 255);

        if version.equals_or_above(1, 5) {

            let _light_longitude = byte_stream.integer32();
            let _light_latitude = byte_stream.integer32();
            let _diffuse_color = byte_stream.color();
            let _ambient_color = byte_stream.color();

            ambient_light_color = _ambient_color;

            #[cfg(feature = "debug_map")]
            {
                print_debug!("light longitude {}{}{}", magenta(), _light_longitude, none());
                print_debug!("light latitude {}{}{}", magenta(), _light_latitude, none());
                print_debug!("diffuse color {}{:?}{}", magenta(), _diffuse_color, none());
                print_debug!("ambient color {}{:?}{}", magenta(), _ambient_color, none());
            }
        }

        if version.equals_or_above(1, 7) {
            let _unknown = byte_stream.float32();
        }

        if version.equals_or_above(1, 6) {

            let _ground_top = byte_stream.integer32();
            let _ground_bottom = byte_stream.integer32();
            let _ground_left = byte_stream.integer32();
            let _ground_right = byte_stream.integer32();

            #[cfg(feature = "debug_map")]
            {
                print_debug!("ground top {}{}{}", magenta(), _ground_right, none());
                print_debug!("ground bottom {}{}{}", magenta(), _ground_left, none());
                print_debug!("ground left {}{}{}", magenta(), _ground_bottom, none());
                print_debug!("ground right {}{}{}", magenta(), _ground_top, none());
            }
        }

        let object_count = byte_stream.integer32() as usize;

        #[cfg(feature = "debug_map")]
        print_debug!("object count {}{}{}", magenta(), object_count, none());

        let mut objects = Vec::new();
        let mut light_sources = Vec::new();
        let mut sound_sources = Vec::new();
        let mut effect_sources = Vec::new();

        for _index in 0..object_count {
            let type_index = byte_stream.integer32();
            let resource_type = ResourceType::from(type_index);

            match resource_type {

                ResourceType::Object => {

                    if version.equals_or_above(1, 6) {

                        let _name = byte_stream.string(40);
                        let _animation_type = byte_stream.integer32();
                        let _animation_speed = byte_stream.float32();
                        let _block_type = byte_stream.integer32();
                        let model_name = byte_stream.string(80);
                        let _node_name = byte_stream.string(80);
                        let position = byte_stream.vector3();
                        let rotation = byte_stream.vector3();
                        let scale = byte_stream.vector3();

                        let model_name_unix = model_name.replace("\\", "/");
                        let model = model_loader.get(texture_loader, format!("data/model/{}", model_name_unix), &mut texture_future);

                        let transform = Transform::from(position, rotation.map(|value| Deg(value)), scale);

                        let object = Object::new(model, transform);
                        objects.push(object);

                        #[cfg(feature = "debug_map")]
                        {
                            print_debug!("name {}{}{}", magenta(), _name, none());
                            print_debug!("animation_type {}{}{}", magenta(), _animation_type, none());
                            print_debug!("animation_speed {}{}{}", magenta(), _animation_speed, none());
                            print_debug!("block type {}{}{}", magenta(), _block_type, none());
                            print_debug!("model name {}{}{}", magenta(), model_name, none());
                            print_debug!("node name {}{}{}", magenta(), _node_name, none());
                            print_debug!("position {}{:?}{}", magenta(), position, none());
                            print_debug!("rotation {}{:?}{}", magenta(), rotation, none());
                            print_debug!("scale {}{:?}{}", magenta(), scale, none());
                        }
                    } else {

                        let model_name = byte_stream.string(80);
                        let _node_name = byte_stream.string(80);
                        let position = byte_stream.vector3();
                        let rotation = byte_stream.vector3();
                        let scale = byte_stream.vector3();

                        let model_name_unix = model_name.replace("\\", "/");
                        let model = model_loader.get(texture_loader, format!("data/model/{}", model_name_unix), &mut texture_future);

                        let transform = Transform::from(position, rotation.map(|value| Deg(value)), scale);

                        objects.push(Object::new(model, transform));

                        #[cfg(feature = "debug_map")]
                        {
                            print_debug!("model name {}{}{}", magenta(), model_name, none());
                            print_debug!("node name {}{}{}", magenta(), _node_name, none());
                            print_debug!("position {}{:?}{}", magenta(), position, none());
                            print_debug!("rotation {}{:?}{}", magenta(), rotation, none());
                            print_debug!("scale {}{:?}{}", magenta(), scale, none());
                        }
                    }
                },

                ResourceType::LightSource => {

                    let _name = byte_stream.string(80);
                    let position = byte_stream.vector3_flipped();

                    let red = byte_stream.integer32() as u8;
                    let green = byte_stream.integer32() as u8;
                    let blue = byte_stream.integer32() as u8;
                    let color = Color::new(red, green, blue);

                    let range = byte_stream.float32();

                    #[cfg(feature = "debug_map")]
                    {
                        print_debug!("name {}{}{}", magenta(), _name, none());
                        print_debug!("position {}{:?}{}", magenta(), position, none());
                        print_debug!("color {}{:?}{}", magenta(), color, none());
                        print_debug!("range {}{}{}", magenta(), range, none());
                    }

                    light_sources.push(LightSource::new(position, color, range));
                },

                ResourceType::SoundSource => {

                    let _name = byte_stream.string(80);
                    let _wave_name = byte_stream.string(80);
                    let position = byte_stream.vector3_flipped();
                    let _volume = byte_stream.float32();
                    let _width = byte_stream.integer32();
                    let _height = byte_stream.integer32();
                    let range = byte_stream.float32();

                    let _cycle = match version.equals_or_above(2, 0) {
                        true => byte_stream.float32(),
                        false => 4.0,
                    };

                    sound_sources.push(SoundSource::new(position, range));

                    #[cfg(feature = "debug_map")]
                    {
                        print_debug!("name {}{}{}", magenta(), _name, none());
                        print_debug!("wave name {}{}{}", magenta(), _wave_name, none());
                        print_debug!("position {}{:?}{}", magenta(), position, none());
                        print_debug!("volume {}{}{}", magenta(), _volume, none());
                        print_debug!("width {}{}{}", magenta(), _width, none());
                        print_debug!("height {}{}{}", magenta(), _height, none());
                        print_debug!("range {}{}{}", magenta(), range, none());
                        print_debug!("cycle {}{}{}", magenta(), _cycle, none());
                    }

                },

                ResourceType::EffectSource => {

                    let _name = byte_stream.string(80);
                    let position = byte_stream.vector3();
                    let _effect_type = byte_stream.integer32();
                    let _emit_speed = byte_stream.float32();

                    let _param0 = byte_stream.float32();
                    let _param1 = byte_stream.float32();
                    let _param2 = byte_stream.float32();
                    let _param3 = byte_stream.float32();

                    effect_sources.push(EffectSource::new(position));

                    #[cfg(feature = "debug_map")]
                    {
                        print_debug!("name {}{}{}", magenta(), _name, none());
                        print_debug!("position {}{:?}{}", magenta(), position, none());
                        print_debug!("effect type {}{}{}", magenta(), _effect_type, none());
                        print_debug!("emit speed {}{}{}", magenta(), _emit_speed, none());
                        print_debug!("param0 {}{}{}", magenta(), _param0, none());
                        print_debug!("param1 {}{}{}", magenta(), _param1, none());
                        print_debug!("param2 {}{}{}", magenta(), _param2, none());
                        print_debug!("param3 {}{}{}", magenta(), _param3, none());
                    }
                },
            }
        }

        #[cfg(feature = "debug")]
        byte_stream.assert_empty(bytes.len(), &resource_file);

        let bytes = read(ground_file.clone()).expect("u r stupid");
        let mut byte_stream = ByteStream::new(bytes.iter());

        let magic = byte_stream.string(4);
        assert!(&magic == "GRGN", "failed to read magic number");
        let version = byte_stream.version();

        if !version.equals_or_above(1, 6) {
            panic!("failed to read ground version");
        }

        let width = byte_stream.integer32() as usize;
        let height = byte_stream.integer32() as usize;
        let _zoom = byte_stream.float32();
        let texture_count = byte_stream.integer32();
        let texture_name_length = byte_stream.integer32();

        let mut textures = Vec::new();

        for _index in 0..texture_count {
            let texture_name = byte_stream.string(texture_name_length as usize);
            let texture_name_unix = texture_name.replace("\\", "/");
            let full_name = format!("data/texture/{}", texture_name_unix);
            let texture = texture_loader.get(full_name, &mut texture_future);
            textures.push(texture);
        }

        let light_map_count = byte_stream.integer32() as usize;
        let light_map_width = byte_stream.integer32() as usize;
        let light_map_height = byte_stream.integer32() as usize;
        let _light_map_cells_per_grid = byte_stream.integer32();

        let dimensions = width * height;
        let light_map_dimensions = light_map_width * light_map_height;

        match version.equals_or_above(1, 7) {
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

            let color = Color::new(color_bgra[2], color_bgra[1], color_bgra[0]);
            surfaces.push(Surface::new(u, v, texture_index, light_map_index, color));
        }

        let mut ground_tiles = Vec::new();

        for _index in 0..dimensions {

            let upper_left_height = byte_stream.float32();
            let upper_right_height = byte_stream.float32();
            let lower_left_height = byte_stream.float32();
            let lower_right_height = byte_stream.float32();

            let top_surface_index = match version.equals_or_above(1, 7) {
                true => byte_stream.integer32(),
                false => byte_stream.integer16() as i32,
            };

            let front_surface_index = match version.equals_or_above(1, 7) {
                true => byte_stream.integer32(),
                false => byte_stream.integer16() as i32,
            };

            let right_surface_index = match version.equals_or_above(1, 7) {
                true => byte_stream.integer32(),
                false => byte_stream.integer16() as i32,
            };

            ground_tiles.push(GroundTile::new(upper_left_height, upper_right_height, lower_left_height, lower_right_height, top_surface_index, front_surface_index, right_surface_index));
        }

        #[cfg(feature = "debug")]
        byte_stream.assert_empty(bytes.len(), &ground_file);

        let mut map_width = width;
        let mut map_height = height;
        let mut tiles = Vec::new();
        let mut tile_vertex_buffer = None;

        if let Some(gat_file) = gat_file {

            let bytes = read(gat_file.clone()).expect("u r very very stupid");
            let mut byte_stream = ByteStream::new(bytes.iter());

            let magic = byte_stream.string(4);
            assert!(&magic == "GRAT", "failed to read magic number");

            let version = byte_stream.version();

            if !version.equals(1, 2) {
                panic!("invalid gat version");
            }

            map_width = byte_stream.integer32() as usize; // todo: unsigned
            map_height = byte_stream.integer32() as usize; // todo: unsigned

            let mut tile_vertices = Vec::new();

            /* */
            let mut vertex_offset = 1;
            let mut obj_file_vertices = String::new();
            let mut obj_file_faces = String::new();
            /* */

            for y in 0..map_height {
                for x in 0..map_width {

                    let upper_left_height = byte_stream.float32();
                    let upper_right_height = byte_stream.float32();
                    let lower_left_height = byte_stream.float32();
                    let lower_right_height = byte_stream.float32();
                    let tile_type_index = byte_stream.byte();
                    let tile_type = TileType::new(tile_type_index);

                    // unknown
                    byte_stream.skip(3);

                    tiles.push(Tile::new(upper_left_height, upper_right_height, lower_left_height, lower_right_height, tile_type));

                    if tile_type.is_none() {
                        continue;
                    }

                    let offset = Vector2::new(x as f32 * 5.0, y as f32 * 5.0);

                    let first_position = Vector3::new(offset.x, -upper_left_height + 1.0, offset.y);
                    let second_position = Vector3::new(offset.x + 5.0, -upper_right_height + 1.0, offset.y);
                    let third_position = Vector3::new(offset.x + 5.0, -lower_right_height + 1.0, offset.y + 5.0);
                    let fourth_position = Vector3::new(offset.x, -lower_left_height + 1.0, offset.y + 5.0);

                    /*  */
                    obj_file_vertices.push_str(&format!("v {} {} {}\n", first_position.x, first_position.y, first_position.z));
                    obj_file_vertices.push_str(&format!("v {} {} {}\n", second_position.x, second_position.y, second_position.z));
                    obj_file_vertices.push_str(&format!("v {} {} {}\n", third_position.x, third_position.y, third_position.z));
                    obj_file_vertices.push_str(&format!("v {} {} {}\n", fourth_position.x, fourth_position.y, fourth_position.z));
                    /*  */

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

                    /*  */
                    obj_file_faces.push_str(&format!("f {} {} {}\n", vertex_offset, vertex_offset + 1, vertex_offset + 2));
                    obj_file_faces.push_str(&format!("f {} {} {}\n", vertex_offset, vertex_offset + 2, vertex_offset + 3));
                    vertex_offset += 4;
                    /*  */
                }
            }

            /* */
            use std::fs::File;
            use std::io::prelude::*;
            let mut file = File::create(&format!("{}.obj", gat_file.clone())).unwrap();
            file.write_all(obj_file_vertices.as_bytes()).unwrap();
            file.write_all(obj_file_faces.as_bytes()).unwrap();
            /* */

            #[cfg(feature = "debug")]
            byte_stream.assert_empty(bytes.len(), &gat_file);

            let vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, tile_vertices.into_iter()).unwrap();
            tile_vertex_buffer = Some(vertex_buffer);
        }

        let mut native_ground_vertices = Vec::new();

        /* */
        let mut vertex_offset = 1;
        let mut obj_file_vertices = String::new();
        let mut obj_file_faces = String::new();
        /* */

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
                        let height = get_tile_height_at(&current_tile, surface_height);
                        let first_position = Vector3::new((x + surface_offset.x) as f32 * TILE_SIZE, -height, (y + surface_offset.y) as f32 * TILE_SIZE);

                        let (surface_offset, surface_height) = surface_alignment[1];
                        let height = get_tile_height_at(&current_tile, surface_height);
                        let second_position = Vector3::new((x + surface_offset.x) as f32 * TILE_SIZE, -height, (y + surface_offset.y) as f32 * TILE_SIZE);

                        let (surface_offset, surface_height) = surface_alignment[2];
                        let height = get_tile_height_at(&neighbor_tile, surface_height);
                        let third_position = Vector3::new((x + surface_offset.x) as f32 * TILE_SIZE, -height, (y + surface_offset.y) as f32 * TILE_SIZE);

                        let (surface_offset, surface_height) = surface_alignment[3];
                        let height = get_tile_height_at(&neighbor_tile, surface_height);
                        let fourth_position = Vector3::new((x + surface_offset.x) as f32 * TILE_SIZE, -height, (y + surface_offset.y) as f32 * TILE_SIZE);

                        /*  */
                        obj_file_vertices.push_str(&format!("v {} {} {}\n", first_position.x, first_position.y, first_position.z));
                        obj_file_vertices.push_str(&format!("v {} {} {}\n", second_position.x, second_position.y, second_position.z));
                        obj_file_vertices.push_str(&format!("v {} {} {}\n", third_position.x, third_position.y, third_position.z));
                        obj_file_vertices.push_str(&format!("v {} {} {}\n", fourth_position.x, fourth_position.y, fourth_position.z));
                        /*  */

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

                        /*  */
                        obj_file_faces.push_str(&format!("f {} {} {}\n", vertex_offset, vertex_offset + 1, vertex_offset + 2));
                        obj_file_faces.push_str(&format!("f {} {} {}\n", vertex_offset, vertex_offset + 2, vertex_offset + 3));
                        vertex_offset += 4;
                        /*  */
                    }
                }
            }
        }

        /* */
        use std::fs::File;
        use std::io::prelude::*;
        let mut file = File::create(&format!("{}.obj", resource_file.clone())).unwrap();
        file.write_all(obj_file_vertices.as_bytes()).unwrap();
        file.write_all(obj_file_faces.as_bytes()).unwrap();
        /* */

        let row_size = width * 6;

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
        }

        //for ModelVertex in &mut native_ground_vertices {
        //    let array = &ModelVertex.normal;
        //    let new_normal = Vector3::new(array[0], array[1], array[2]).normalize();
        //    ModelVertex.normal = [new_normal.x, new_normal.y, new_normal.z];
        //}

        let ground_vertices = NativeModelVertex::to_vertices(native_ground_vertices);
        let ground_vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, ground_vertices.into_iter()).unwrap();

        #[cfg(feature = "debug_map")]
        {
            print_debug!("version {}{}{}", magenta(), version, none());
            print_debug!("width {}{}{}", magenta(), width, none());
            print_debug!("height {}{}{}", magenta(), height, none());
            print_debug!("zoom {}{}{}", magenta(), _zoom, none());
            print_debug!("texture count {}{}{}", magenta(), texture_count, none());
            print_debug!("texture name length {}{}{}", magenta(), texture_name_length, none());

            print_debug!("light map count {}{}{}", magenta(), light_map_count, none());
            print_debug!("light map width {}{}{}", magenta(), light_map_width, none());
            print_debug!("light map height {}{}{}", magenta(), light_map_height, none());
            print_debug!("light map cells per grid {}{}{}", magenta(), _light_map_cells_per_grid, none());

            print_debug!("surface count {}{}{}", magenta(), surface_count, none());
        }

        let offset = Vector3::new(width as f32 * MAP_OFFSET, 0.0, height as f32 * MAP_OFFSET);
        objects.iter_mut().for_each(|object| object.offset(offset));
        light_sources.iter_mut().for_each(|light_source| light_source.offset(offset));
        sound_sources.iter_mut().for_each(|sound_source| sound_source.offset(offset));
        effect_sources.iter_mut().for_each(|effect_source| effect_source.offset(offset));

        let map = Arc::new(Map::new(map_width, map_height, tiles, ground_vertex_buffer, textures, objects, light_sources, sound_sources, effect_sources, tile_vertex_buffer, ambient_light_color));

        self.cache.insert(resource_file, map.clone());

        texture_future.flush().unwrap();
        texture_future.cleanup_finished();

        #[cfg(feature = "debug")]
        timer.stop();

        return map;
    }

    pub fn get(&mut self, model_loader: &mut ModelLoader, texture_loader: &mut TextureLoader, resource_file: String) -> Arc<Map> {
        match self.cache.get(&resource_file) {
            Some(map) => return map.clone(),
            None => return self.load(model_loader, texture_loader, resource_file),
        }
    }
}
