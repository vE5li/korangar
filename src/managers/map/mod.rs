mod map;

use std::sync::Arc;
use std::collections::HashMap;
use std::fs::read;

use cgmath::{ Vector3, Vector2, InnerSpace };

use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::device::Device;

#[cfg(feature = "debug")]
use debug::*;

use managers::{ ModelManager, TextureManager };
use graphics::{ Color, Vertex };

use super::ByteStream;

use self::map::Map;

pub struct Surface {
    u: [f32; 4],
    v: [f32; 4],
    texture_index: i32,
    light_map_index: i32,
    color: Color,
}

impl Surface {

    pub fn new(u: [f32; 4], v: [f32; 4], texture_index: i32, light_map_index: i32, color: Color) -> Self {
        return Self { u, v, texture_index, light_map_index, color };
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SurfaceType {
    Front,
    Right,
    Top
}

#[derive(Copy, Clone, Debug)]
pub enum Heights {
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight
}

pub struct Tile {
    pub upper_left_height: f32,
    pub upper_right_height: f32,
    pub lower_left_height: f32,
    pub lower_right_height: f32,
    pub top_surface_index: i32,
    pub front_surface_index: i32,
    pub right_surface_index: i32,
}

impl Tile {

    pub fn new(upper_left_height: f32, upper_right_height: f32, lower_left_height: f32, lower_right_height: f32, top_surface_index: i32, front_surface_index: i32, right_surface_index: i32) -> Self {
        return Self { upper_left_height, upper_right_height, lower_left_height, lower_right_height, top_surface_index, front_surface_index, right_surface_index };
    }

    pub fn surface_index(&self, surface_type: SurfaceType) -> i32 {
        match surface_type {
            SurfaceType::Front => return self.front_surface_index,
            SurfaceType::Right => return self.right_surface_index,
            SurfaceType::Top => return self.top_surface_index,
        }
    }

    pub fn get_height_at(&self, point: Heights) -> f32 {
        match point {
            Heights::UpperLeft => return self.upper_left_height,
            Heights::UpperRight => return self.upper_right_height,
            Heights::LowerLeft => return self.lower_left_height,
            Heights::LowerRight => return self.lower_right_height,
        }
    }

    pub fn surface_alignment(surface_type: SurfaceType) -> [(Vector2<usize>, Heights); 4] {
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
}

pub fn calculate_normal(first_position: Vector3<f32>, second_position: Vector3<f32>, third_position: Vector3<f32>) -> Vector3<f32> {
    let delta_position_1 = second_position - first_position;
    let delta_position_2 = third_position - first_position;
    return delta_position_1.cross(delta_position_2);
}

pub struct MapManager {
    cache: HashMap<String, Arc<Map>>,
    device: Arc<Device>,
}

impl MapManager {

    pub fn new(device: Arc<Device>) -> Self {
        return Self {
            cache: HashMap::new(),
            device: device,
        }
    }

    fn load(&mut self, model_manager: &mut ModelManager, texture_manager: &mut TextureManager, path: String) -> Arc<Map> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load map from {}{}{}", magenta(), path, none()));

        let bytes = read(path.clone()).expect("u r stupid");
        let mut byte_stream = ByteStream::new(bytes.iter());

        // ground

        let magic = byte_stream.string(4);
        assert!(&magic == "GRGN", "failed to read magic number");
        let version = byte_stream.version();

        if !version.equals_or_above(1, 6) {
            panic!("failed to read ground version");
        }

        let width = byte_stream.integer(4) as usize;
        let height = byte_stream.integer(4) as usize;
        let zoom = byte_stream.float32();
        let texture_count = byte_stream.integer(4);
        let texture_name_length = byte_stream.integer(4);

        let mut textures = Vec::new();

        for _index in 0..texture_count {
            let texture_name = byte_stream.string(texture_name_length as usize);
            let texture_name_unix = texture_name.replace("\\", "/");
            let full_name = format!("data/texture/{}", texture_name_unix);
            let (texture, mut future) = texture_manager.get(full_name);

            // todo return gpu future instead
            future.cleanup_finished();
            textures.push(texture);
        }

        let light_map_count = byte_stream.integer(4) as usize;
        let light_map_width = byte_stream.integer(4) as usize;
        let light_map_height = byte_stream.integer(4) as usize;
        let light_map_cells_per_grid = byte_stream.integer(4);

        let dimensions = width * height;
        let light_map_dimensions = light_map_width * light_map_height;

        match version.equals_or_above(1, 7) {
            true => byte_stream.skip(light_map_count * light_map_dimensions * 4),
            false => byte_stream.skip(light_map_count * 16),
        }

        let surface_count = byte_stream.integer(4);
        let mut surfaces = Vec::new();

        for _index in 0..surface_count {

            let u = [byte_stream.float32(), byte_stream.float32(), byte_stream.float32(), byte_stream.float32()];
            let v = [byte_stream.float32(), byte_stream.float32(), byte_stream.float32(), byte_stream.float32()];

            let texture_index = byte_stream.integer(2) as i32;
            let light_map_index = byte_stream.integer(2) as i32;
            let color_bgra = byte_stream.slice(4);

            let color = Color::new(color_bgra[2], color_bgra[1], color_bgra[0]);
            surfaces.push(Surface::new(u, v, texture_index, light_map_index, color));
        }

        let mut tiles = Vec::new();

        for _index in 0..dimensions {

            let upper_left_height = byte_stream.float32();
            let upper_right_height = byte_stream.float32();
            let lower_left_height = byte_stream.float32();
            let lower_right_height = byte_stream.float32();

            let top_surface_index = match version.equals_or_above(1, 7) {
                true => byte_stream.integer(4) as i32,
                false => byte_stream.integer(2) as i32,
            };

            let front_surface_index = match version.equals_or_above(1, 7) {
                true => byte_stream.integer(4) as i32,
                false => byte_stream.integer(2) as i32,
            };

            let right_surface_index = match version.equals_or_above(1, 7) {
                true => byte_stream.integer(4) as i32,
                false => byte_stream.integer(2) as i32,
            };

            tiles.push(Tile::new(upper_left_height, upper_right_height, lower_left_height, lower_right_height, top_surface_index, front_surface_index, right_surface_index));
        }

        let mut ground_vertices = Vec::new();

        for x in 0..width {
            for y in 0..height {
                let current_tile = &tiles[x + y * width];

                for surface_type in [SurfaceType::Front, SurfaceType::Right, SurfaceType::Top].iter() {
                    let surface_index = current_tile.surface_index(*surface_type);

                    if surface_index > -1 {

                        let surface_alignment = Tile::surface_alignment(*surface_type);
                        let neighbor_tile_index = Tile::neighbor_tile_index(*surface_type);

                        let neighbor_x = x + neighbor_tile_index.x;
                        let neighbor_y = y + neighbor_tile_index.y;
                        let neighbor_tile = &tiles[neighbor_x + neighbor_y * width];

                        let (surface_offset, surface_height) = surface_alignment[0];
                        let height = current_tile.get_height_at(surface_height);
                        let first_position = Vector3::new((x + surface_offset.x) as f32 * 12.0, -height, (y + surface_offset.y) as f32 * 12.0);

                        let (surface_offset, surface_height) = surface_alignment[1];
                        let height = current_tile.get_height_at(surface_height);
                        let second_position = Vector3::new((x + surface_offset.x) as f32 * 12.0, -height, (y + surface_offset.y) as f32 * 12.0);

                        let (surface_offset, surface_height) = surface_alignment[2];
                        let height = neighbor_tile.get_height_at(surface_height);
                        let third_position = Vector3::new((x + surface_offset.x) as f32 * 12.0, -height, (y + surface_offset.y) as f32 * 12.0);

                        let (surface_offset, surface_height) = surface_alignment[3];
                        let height = neighbor_tile.get_height_at(surface_height);
                        let fourth_position = Vector3::new((x + surface_offset.x) as f32 * 12.0, -height, (y + surface_offset.y) as f32 * 12.0);

                        let first_normal = calculate_normal(first_position, second_position, third_position);
                        let second_normal = calculate_normal(fourth_position, first_position, third_position);

                        let ground_surface = &surfaces[surface_index as usize];

                        let first_texture_coordinates = Vector2::new(ground_surface.u[0], ground_surface.v[0]);
                        let second_texture_coordinates = Vector2::new(ground_surface.u[1], ground_surface.v[1]);
                        let third_texture_coordinates = Vector2::new(ground_surface.u[3], ground_surface.v[3]);
                        let fourth_texture_coordinates = Vector2::new(ground_surface.u[2], ground_surface.v[2]);

                        ground_vertices.push(Vertex::new(first_position, first_normal, first_texture_coordinates, ground_surface.texture_index));
                        ground_vertices.push(Vertex::new(second_position, first_normal, second_texture_coordinates, ground_surface.texture_index));
                        ground_vertices.push(Vertex::new(third_position, first_normal, third_texture_coordinates, ground_surface.texture_index));

                        ground_vertices.push(Vertex::new(first_position, second_normal, first_texture_coordinates, ground_surface.texture_index));
                        ground_vertices.push(Vertex::new(third_position, second_normal, third_texture_coordinates, ground_surface.texture_index));
                        ground_vertices.push(Vertex::new(fourth_position, second_normal, fourth_texture_coordinates, ground_surface.texture_index));
                    }
                }
            }
        }

        let row_size = width * 6;

        for index in 0..ground_vertices.len() / 6 {

            let base_index = index * 6;
            let mut indices = vec![base_index + 5];

            if base_index + 6 < ground_vertices.len() {
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
                .map(|index| ground_vertices[*index].normal)
                .map(|array| Vector3::new(array[0], array[1], array[2]))
                .fold(Vector3::new(0.0, 0.0, 0.0), |sum, normal| sum + normal);

            indices.iter().for_each(|index| ground_vertices[*index].normal = [new_normal.x, new_normal.y, new_normal.z]);
        }

        for vertex in &mut ground_vertices {
            let array = &vertex.normal;
            let new_normal = Vector3::new(array[0], array[1], array[2]).normalize();
            vertex.normal = [new_normal.x, new_normal.y, new_normal.z];
        }

        let ground_vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, ground_vertices.into_iter()).unwrap();

        #[cfg(feature = "debug_map")]
        {
            print_debug!("version {}{}{}", magenta(), version, none());
            print_debug!("width {}{}{}", magenta(), width, none());
            print_debug!("height {}{}{}", magenta(), height, none());
            print_debug!("zoom {}{}{}", magenta(), zoom, none());
            print_debug!("texture count {}{}{}", magenta(), texture_count, none());
            print_debug!("texture name length {}{}{}", magenta(), texture_name_length, none());

            print_debug!("light map count {}{}{}", magenta(), light_map_count, none());
            print_debug!("light map width {}{}{}", magenta(), light_map_width, none());
            print_debug!("light map height {}{}{}", magenta(), light_map_height, none());
            print_debug!("light map cells per grid {}{}{}", magenta(), light_map_cells_per_grid, none());

            print_debug!("surface count {}{}{}", magenta(), surface_count, none());

            let remaining = byte_stream.remaining(bytes.len());
            print_debug!("remaining {}{}{}", magenta(), remaining, none());
        }

        // terrain

        // resources

        //let magic = byte_stream.string(4);
        //assert!(&magic == "GRSW", "failed to read magic number");

        // create map

        let map = Arc::new(Map::new(ground_vertex_buffer, textures));

        self.cache.insert(path, map.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        return map;
    }

    pub fn get(&mut self, model_manager: &mut ModelManager, texture_manager: &mut TextureManager, path: String) -> Arc<Map> {
        match self.cache.get(&path) {
            Some(map) => return map.clone(),
            None => return self.load(model_manager, texture_manager, path),
        }
    }
}
