use cgmath::{Point3, Vector2};
use ragnarok_formats::map::{GatData, GroundData, GroundTile, SurfaceType};
use smallvec::smallvec_inline;

#[cfg(feature = "debug")]
use crate::graphics::Color;
use crate::graphics::{ModelVertex, NativeModelVertex, PickerTarget, TileVertex, reduce_vertices};
use crate::loaders::map::{GAT_TILE_SIZE, GROUND_TILE_SIZE};
use crate::loaders::{TextureSetBuilder, smooth_ground_normals};

#[derive(Copy, Clone, Debug)]
pub enum Heights {
    SouthWest,
    SouthEast,
    NorthWest,
    NorthEast,
}

pub fn ground_vertices(ground_data: &GroundData, texture_set_builder: &mut TextureSetBuilder) -> (Vec<ModelVertex>, Vec<u32>, Vec<bool>) {
    let mut ground_vertices = Vec::new();

    let width = ground_data.width as usize;
    let ground_tiles = &ground_data.ground_tiles;

    for (index, current_tile) in ground_tiles.iter().enumerate() {
        let tile_x = index % width;
        let tile_y = index / width;

        for surface_type in [SurfaceType::North, SurfaceType::East, SurfaceType::Top] {
            let surface_index = tile_surface_index(current_tile, surface_type);

            if surface_index.is_negative() {
                continue;
            }

            let surface_alignment = tile_surface_alignment(surface_type);

            let position = |alignment_index: usize, tile_for_height: &GroundTile| {
                let (surface_offset, surface_height) = surface_alignment[alignment_index];
                let height = get_tile_height_at(tile_for_height, surface_height);
                Point3::new(
                    (tile_x + surface_offset.x) as f32 * GROUND_TILE_SIZE,
                    -height,
                    (tile_y + surface_offset.y) as f32 * GROUND_TILE_SIZE,
                )
            };

            let (first_position, second_position, third_position, fourth_position) = match surface_type {
                SurfaceType::North | SurfaceType::East => {
                    let neighbor_tile_index = neighbor_tile_index(surface_type);

                    let neighbor_x = tile_x + neighbor_tile_index.x;
                    let neighbor_y = tile_y + neighbor_tile_index.y;

                    let Some(neighbor_tile) = ground_tiles.get(neighbor_x + neighbor_y * width) else {
                        continue;
                    };

                    (
                        position(0, current_tile),
                        position(1, current_tile),
                        position(2, neighbor_tile),
                        position(3, neighbor_tile),
                    )
                }
                SurfaceType::Top => (
                    position(0, current_tile),
                    position(1, current_tile),
                    position(2, current_tile),
                    position(3, current_tile),
                ),
            };

            let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
            let second_normal = NativeModelVertex::calculate_normal(third_position, second_position, fourth_position);

            let ground_surface = &ground_data.surfaces[surface_index as usize];

            let first_texture_coordinates = Vector2::new(ground_surface.u[0], ground_surface.v[0]);
            let second_texture_coordinates = Vector2::new(ground_surface.u[1], ground_surface.v[1]);
            let third_texture_coordinates = Vector2::new(ground_surface.u[2], ground_surface.v[2]);
            let fourth_texture_coordinates = Vector2::new(ground_surface.u[3], ground_surface.v[3]);

            let neighbor_color = |x_offset: usize, y_offset: usize| {
                let Some(neighbor_tile) = ground_tiles.get(tile_x + x_offset + (tile_y + y_offset) * width) else {
                    return ground_surface.color.into();
                };

                // FIX: It is almost certainly incorrect to use the top face in all cases.
                let neighbor_surface_index = tile_surface_index(neighbor_tile, SurfaceType::Top);
                let Some(neighbor_surface) = ground_data.surfaces.get(neighbor_surface_index as usize) else {
                    return ground_surface.color.into();
                };

                neighbor_surface.color.into()
            };

            let color_east = neighbor_color(1, 0);
            let color_north_east = neighbor_color(1, 1);
            let color_north = neighbor_color(0, 1);

            if let Some(first_normal) = first_normal {
                ground_vertices.push(NativeModelVertex::new(
                    first_position,
                    first_normal,
                    first_texture_coordinates,
                    ground_surface.texture_index as i32,
                    ground_surface.color.into(),
                    0.0,
                    smallvec_inline![0;3],
                ));
                ground_vertices.push(NativeModelVertex::new(
                    second_position,
                    first_normal,
                    second_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_east,
                    0.0,
                    smallvec_inline![0;3],
                ));
                ground_vertices.push(NativeModelVertex::new(
                    third_position,
                    first_normal,
                    third_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_north,
                    0.0,
                    smallvec_inline![0;3],
                ));
            }

            if let Some(second_normal) = second_normal {
                ground_vertices.push(NativeModelVertex::new(
                    third_position,
                    second_normal,
                    third_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_north,
                    0.0,
                    smallvec_inline![0;3],
                ));
                ground_vertices.push(NativeModelVertex::new(
                    second_position,
                    second_normal,
                    second_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_east,
                    0.0,
                    smallvec_inline![0;3],
                ));
                ground_vertices.push(NativeModelVertex::new(
                    fourth_position,
                    second_normal,
                    fourth_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_north_east,
                    0.0,
                    smallvec_inline![0;3],
                ));
            }
        }
    }

    let (ground_texture_mapping, ground_texture_transparencies): (Vec<i32>, Vec<bool>) = ground_data
        .textures
        .iter()
        .map(|texture| texture_set_builder.register(texture))
        .unzip();

    smooth_ground_normals(&mut ground_vertices);

    let vertices = NativeModelVertex::convert_to_model_vertices(ground_vertices, Some(&ground_texture_mapping));

    let (reduced_vertices, indices) = reduce_vertices(&vertices);

    (reduced_vertices, indices, ground_texture_transparencies)
}

pub fn generate_tile_vertices(gat_data: &mut GatData) -> (Vec<ModelVertex>, Vec<u32>, Vec<TileVertex>, Vec<u32>) {
    #[allow(unused_mut)]
    let mut tile_vertices = Vec::new();
    let mut tile_picker_vertices = Vec::new();

    let tile_picker_indices = gat_data
        .tiles
        .iter_mut()
        .enumerate()
        .filter(|(_, tile)| !tile.flags.is_empty())
        .flat_map(|(index, tile)| {
            let x = index % gat_data.map_width as usize;
            let y = index / gat_data.map_width as usize;

            tile.southwest_corner_height = -tile.southwest_corner_height;
            tile.southeast_corner_height = -tile.southeast_corner_height;
            tile.northwest_corner_height = -tile.northwest_corner_height;
            tile.northeast_corner_height = -tile.northeast_corner_height;

            let offset = Vector2::new(x as f32 * GAT_TILE_SIZE, y as f32 * GAT_TILE_SIZE);

            #[cfg(feature = "debug")]
            {
                const TILE_MESH_OFFSET: f32 = 0.9;

                let first_position = Point3::new(offset.x, tile.southwest_corner_height + TILE_MESH_OFFSET, offset.y);
                let second_position = Point3::new(
                    offset.x + GAT_TILE_SIZE,
                    tile.southeast_corner_height + TILE_MESH_OFFSET,
                    offset.y,
                );
                let third_position = Point3::new(
                    offset.x,
                    tile.northwest_corner_height + TILE_MESH_OFFSET,
                    offset.y + GAT_TILE_SIZE,
                );
                let fourth_position = Point3::new(
                    offset.x + GAT_TILE_SIZE,
                    tile.northeast_corner_height + TILE_MESH_OFFSET,
                    offset.y + GAT_TILE_SIZE,
                );

                let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
                let second_normal = NativeModelVertex::calculate_normal(third_position, second_position, fourth_position);

                let tile_type_index = TryInto::<u8>::try_into(tile.flags).unwrap() as usize;

                let first_texture_coordinates = Vector2::new(0.0, 1.0);
                let second_texture_coordinates = Vector2::new(1.0, 1.0);
                let third_texture_coordinates = Vector2::new(0.0, 0.0);
                let fourth_texture_coordinates = Vector2::new(1.0, 0.0);

                if let Some(first_normal) = first_normal {
                    tile_vertices.push(ModelVertex::new(
                        first_position,
                        first_normal,
                        first_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                    tile_vertices.push(ModelVertex::new(
                        second_position,
                        first_normal,
                        second_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                    tile_vertices.push(ModelVertex::new(
                        third_position,
                        first_normal,
                        third_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                }

                if let Some(second_normal) = second_normal {
                    tile_vertices.push(ModelVertex::new(
                        third_position,
                        second_normal,
                        third_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                    tile_vertices.push(ModelVertex::new(
                        second_position,
                        second_normal,
                        second_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                    tile_vertices.push(ModelVertex::new(
                        fourth_position,
                        second_normal,
                        fourth_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                }
            }

            let first_position = Point3::new(offset.x, tile.southwest_corner_height, offset.y);
            let second_position = Point3::new(offset.x + GAT_TILE_SIZE, tile.southeast_corner_height, offset.y);
            let third_position = Point3::new(offset.x, tile.northwest_corner_height, offset.y + GAT_TILE_SIZE);
            let fourth_position = Point3::new(offset.x + GAT_TILE_SIZE, tile.northeast_corner_height, offset.y + GAT_TILE_SIZE);

            let (_, color) = PickerTarget::Tile { x: x as u16, y: y as u16 }.into();

            let offset = tile_picker_vertices.len() as u32;

            tile_picker_vertices.push(TileVertex::new(first_position, color));
            tile_picker_vertices.push(TileVertex::new(second_position, color));
            tile_picker_vertices.push(TileVertex::new(third_position, color));
            tile_picker_vertices.push(TileVertex::new(fourth_position, color));

            // Since the tile position is encoded in the vertex color, vertices of tiles
            // never share vertices, so we know the correct, minimal indices.
            [offset, offset + 1, offset + 2, offset + 2, offset + 1, offset + 3]
        })
        .collect();

    let (reduced_tile_vertices, tile_indices) = reduce_vertices(&tile_vertices);

    (reduced_tile_vertices, tile_indices, tile_picker_vertices, tile_picker_indices)
}

pub fn tile_surface_index(tile: &GroundTile, surface_type: SurfaceType) -> i32 {
    match surface_type {
        SurfaceType::North => tile.north_surface_index,
        SurfaceType::East => tile.east_surface_index,
        SurfaceType::Top => tile.top_surface_index,
    }
}

pub fn get_tile_height_at(tile: &GroundTile, point: Heights) -> f32 {
    match point {
        Heights::SouthWest => tile.southwest_corner_height,
        Heights::SouthEast => tile.southeast_corner_height,
        Heights::NorthWest => tile.northwest_corner_height,
        Heights::NorthEast => tile.northeast_corner_height,
    }
}

pub fn tile_surface_alignment(surface_type: SurfaceType) -> [(Vector2<usize>, Heights); 4] {
    match surface_type {
        SurfaceType::North => [
            (Vector2::new(0, 1), Heights::NorthWest),
            (Vector2::new(1, 1), Heights::NorthEast),
            (Vector2::new(0, 1), Heights::SouthWest),
            (Vector2::new(1, 1), Heights::SouthEast),
        ],
        SurfaceType::East => [
            (Vector2::new(1, 1), Heights::NorthEast),
            (Vector2::new(1, 0), Heights::SouthEast),
            (Vector2::new(1, 1), Heights::NorthWest),
            (Vector2::new(1, 0), Heights::SouthWest),
        ],
        SurfaceType::Top => [
            (Vector2::new(0, 0), Heights::SouthWest),
            (Vector2::new(1, 0), Heights::SouthEast),
            (Vector2::new(0, 1), Heights::NorthWest),
            (Vector2::new(1, 1), Heights::NorthEast),
        ],
    }
}

pub fn neighbor_tile_index(surface_type: SurfaceType) -> Vector2<usize> {
    match surface_type {
        SurfaceType::North => Vector2::new(0, 1),
        SurfaceType::East => Vector2::new(1, 0),
        _ => unreachable!(),
    }
}
