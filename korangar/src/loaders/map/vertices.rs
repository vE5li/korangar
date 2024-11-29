#[cfg(feature = "debug")]
use cgmath::{EuclideanSpace, Point2};
use cgmath::{Point3, Vector2};
#[cfg(feature = "debug")]
use korangar_util::texture_atlas::AtlasAllocation;
use ragnarok_formats::map::{GatData, GroundData, GroundTile, SurfaceType};

use super::GroundTileExt;
#[cfg(feature = "debug")]
use crate::graphics::Color;
use crate::graphics::{ModelVertex, NativeModelVertex, PickerTarget, TileVertex, WaterVertex};

pub const MAP_TILE_SIZE: f32 = 10.0;

#[derive(Copy, Clone, Debug)]
pub enum Heights {
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight,
}

pub fn ground_water_vertices(ground_data: &GroundData, water_level: f32) -> (Vec<NativeModelVertex>, Vec<WaterVertex>) {
    let mut native_ground_vertices = Vec::new();
    let mut water_vertices = Vec::new();

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
                    let first_position = Point3::new(
                        (x + surface_offset.x) as f32 * MAP_TILE_SIZE,
                        -height,
                        (y + surface_offset.y) as f32 * MAP_TILE_SIZE,
                    );

                    let (surface_offset, surface_height) = surface_alignment[1];
                    let height = get_tile_height_at(current_tile, surface_height);
                    let second_position = Point3::new(
                        (x + surface_offset.x) as f32 * MAP_TILE_SIZE,
                        -height,
                        (y + surface_offset.y) as f32 * MAP_TILE_SIZE,
                    );

                    let (surface_offset, surface_height) = surface_alignment[2];
                    let height = get_tile_height_at(neighbor_tile, surface_height);
                    let third_position = Point3::new(
                        (x + surface_offset.x) as f32 * MAP_TILE_SIZE,
                        -height,
                        (y + surface_offset.y) as f32 * MAP_TILE_SIZE,
                    );

                    let (surface_offset, surface_height) = surface_alignment[3];
                    let height = get_tile_height_at(neighbor_tile, surface_height);
                    let fourth_position = Point3::new(
                        (x + surface_offset.x) as f32 * MAP_TILE_SIZE,
                        -height,
                        (y + surface_offset.y) as f32 * MAP_TILE_SIZE,
                    );

                    let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
                    let second_normal = NativeModelVertex::calculate_normal(fourth_position, first_position, third_position);

                    let ground_surface = &ground_data.surfaces[surface_index as usize];

                    let first_texture_coordinates = Vector2::new(ground_surface.u[0], ground_surface.v[0]);
                    let second_texture_coordinates = Vector2::new(ground_surface.u[1], ground_surface.v[1]);
                    let third_texture_coordinates = Vector2::new(ground_surface.u[3], ground_surface.v[3]);
                    let fourth_texture_coordinates = Vector2::new(ground_surface.u[2], ground_surface.v[2]);

                    let neightbor_color = |x_offset, y_offset| {
                        let Some(neighbor_tile) = ground_tiles.get(x + x_offset + (y + y_offset) * width) else {
                            return ground_surface.color.into();
                        };

                        // FIX: It is alomst certainly incorrect to use the top face in all cases.
                        let neighbor_surface_index = tile_surface_index(neighbor_tile, SurfaceType::Top);
                        let Some(neighbor_surface) = ground_data.surfaces.get(neighbor_surface_index as usize) else {
                            return ground_surface.color.into();
                        };

                        neighbor_surface.color.into()
                    };

                    let color_right = neightbor_color(1, 0);
                    let color_top_right = neightbor_color(1, 1);
                    let color_top = neightbor_color(0, 1);

                    native_ground_vertices.push(NativeModelVertex::new(
                        first_position,
                        first_normal,
                        first_texture_coordinates,
                        ground_surface.texture_index as i32,
                        ground_surface.color.into(),
                        0.0,
                    ));
                    native_ground_vertices.push(NativeModelVertex::new(
                        second_position,
                        first_normal,
                        second_texture_coordinates,
                        ground_surface.texture_index as i32,
                        color_right,
                        0.0,
                    ));
                    native_ground_vertices.push(NativeModelVertex::new(
                        third_position,
                        first_normal,
                        third_texture_coordinates,
                        ground_surface.texture_index as i32,
                        color_top_right,
                        0.0,
                    ));

                    native_ground_vertices.push(NativeModelVertex::new(
                        first_position,
                        second_normal,
                        first_texture_coordinates,
                        ground_surface.texture_index as i32,
                        ground_surface.color.into(),
                        0.0,
                    ));
                    native_ground_vertices.push(NativeModelVertex::new(
                        third_position,
                        second_normal,
                        third_texture_coordinates,
                        ground_surface.texture_index as i32,
                        color_top_right,
                        0.0,
                    ));
                    native_ground_vertices.push(NativeModelVertex::new(
                        fourth_position,
                        second_normal,
                        fourth_texture_coordinates,
                        ground_surface.texture_index as i32,
                        color_top,
                        0.0,
                    ));
                }
            }

            if -current_tile.get_lowest_point() < water_level {
                let first_position = Point3::new(x as f32 * MAP_TILE_SIZE, water_level, y as f32 * MAP_TILE_SIZE);
                let second_position = Point3::new(MAP_TILE_SIZE + x as f32 * MAP_TILE_SIZE, water_level, y as f32 * MAP_TILE_SIZE);
                let third_position = Point3::new(
                    MAP_TILE_SIZE + x as f32 * MAP_TILE_SIZE,
                    water_level,
                    MAP_TILE_SIZE + y as f32 * MAP_TILE_SIZE,
                );
                let fourth_position = Point3::new(x as f32 * MAP_TILE_SIZE, water_level, MAP_TILE_SIZE + y as f32 * MAP_TILE_SIZE);

                water_vertices.push(WaterVertex::new(first_position));
                water_vertices.push(WaterVertex::new(second_position));
                water_vertices.push(WaterVertex::new(third_position));

                water_vertices.push(WaterVertex::new(first_position));
                water_vertices.push(WaterVertex::new(third_position));
                water_vertices.push(WaterVertex::new(fourth_position));
            }
        }
    }
    (native_ground_vertices, water_vertices)
}

pub fn generate_tile_vertices(
    gat_data: &mut GatData,
    #[cfg(feature = "debug")] tile_texture_mapping: &[AtlasAllocation],
) -> (Vec<ModelVertex>, Vec<TileVertex>) {
    const HALF_TILE_SIZE: f32 = MAP_TILE_SIZE / 2.0;

    #[allow(unused_mut)]
    let mut tile_vertices = Vec::new();
    let mut tile_picker_vertices = Vec::new();

    let mut count = 0;
    for y in 0..gat_data.map_height {
        for x in 0..gat_data.map_width {
            let tile = &mut gat_data.tiles[count];

            tile.upper_left_height = -tile.upper_left_height;
            tile.upper_right_height = -tile.upper_right_height;
            tile.lower_left_height = -tile.lower_left_height;
            tile.lower_right_height = -tile.lower_right_height;
            count += 1;

            if tile.flags.is_empty() {
                continue;
            }

            let offset = Vector2::new(x as f32 * HALF_TILE_SIZE, y as f32 * HALF_TILE_SIZE);

            #[cfg(feature = "debug")]
            {
                const TILE_MESH_OFFSET: f32 = 0.9;

                let first_position = Point3::new(offset.x, tile.upper_left_height + TILE_MESH_OFFSET, offset.y);
                let second_position = Point3::new(offset.x + HALF_TILE_SIZE, tile.upper_right_height + TILE_MESH_OFFSET, offset.y);
                let third_position = Point3::new(
                    offset.x + HALF_TILE_SIZE,
                    tile.lower_right_height + TILE_MESH_OFFSET,
                    offset.y + HALF_TILE_SIZE,
                );
                let fourth_position = Point3::new(offset.x, tile.lower_left_height + TILE_MESH_OFFSET, offset.y + HALF_TILE_SIZE);

                let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
                let second_normal = NativeModelVertex::calculate_normal(fourth_position, first_position, third_position);

                let tile_type_index = TryInto::<u8>::try_into(tile.flags).unwrap() as usize;
                let atlas_allocation = tile_texture_mapping[tile_type_index];

                let first_texture_coordinates = atlas_allocation.map_to_atlas(Point2::new(0.0, 0.0)).to_vec();
                let second_texture_coordinates = atlas_allocation.map_to_atlas(Point2::new(0.0, 1.0)).to_vec();
                let third_texture_coordinates = atlas_allocation.map_to_atlas(Point2::new(1.0, 1.0)).to_vec();
                let fourth_texture_coordinates = atlas_allocation.map_to_atlas(Point2::new(1.0, 0.0)).to_vec();

                tile_vertices.push(ModelVertex::new(
                    first_position,
                    first_normal,
                    first_texture_coordinates,
                    Color::WHITE,
                    0.0,
                ));
                tile_vertices.push(ModelVertex::new(
                    second_position,
                    first_normal,
                    second_texture_coordinates,
                    Color::WHITE,
                    0.0,
                ));
                tile_vertices.push(ModelVertex::new(
                    third_position,
                    first_normal,
                    third_texture_coordinates,
                    Color::WHITE,
                    0.0,
                ));

                tile_vertices.push(ModelVertex::new(
                    first_position,
                    second_normal,
                    first_texture_coordinates,
                    Color::WHITE,
                    0.0,
                ));
                tile_vertices.push(ModelVertex::new(
                    third_position,
                    second_normal,
                    third_texture_coordinates,
                    Color::WHITE,
                    0.0,
                ));
                tile_vertices.push(ModelVertex::new(
                    fourth_position,
                    second_normal,
                    fourth_texture_coordinates,
                    Color::WHITE,
                    0.0,
                ));
            }

            let first_position = Point3::new(offset.x, tile.upper_left_height, offset.y);
            let second_position = Point3::new(offset.x + HALF_TILE_SIZE, tile.upper_right_height, offset.y);
            let third_position = Point3::new(offset.x + HALF_TILE_SIZE, tile.lower_right_height, offset.y + HALF_TILE_SIZE);
            let fourth_position = Point3::new(offset.x, tile.lower_left_height, offset.y + HALF_TILE_SIZE);

            let (_, color) = PickerTarget::Tile { x: x as u16, y: y as u16 }.into();
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
