use std::sync::Arc;

use image::{Rgba, RgbaImage};
use ragnarok_formats::map::{Tile, TileFlags};

use crate::graphics::Texture;
use crate::loaders::TextureLoader;
use crate::state::MinimapState;
use crate::world::Map;

pub fn create_minimap_state(texture_loader: &TextureLoader, previous: &MinimapState, map_name: &str, map: &Map) -> MinimapState {
    let texture = create_generated_minimap_texture(texture_loader, map_name, map);

    let arrow_texture = texture_loader.get_or_load("arrow_left.png", ImageType::Sdf).ok();

    MinimapState {
        map_name: map_name.strip_suffix(".gat").unwrap_or(map_name).to_owned(),
        width: map.get_width(),
        height: map.get_height(),
        zoom: previous.zoom.max(1.0),
        texture: Some(texture),
        arrow_texture,
    }
}

fn create_generated_minimap_texture(texture_loader: &TextureLoader, map_name: &str, map: &Map) -> Arc<Texture> {
    let width = u32::from(map.get_width().max(1));
    let height = u32::from(map.get_height().max(1));
    let mut image = RgbaImage::new(width, height);
    let (lowest_height, highest_height) = map
        .get_tiles()
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lowest_height, highest_height), tile| {
            let height = average_tile_height(tile);
            (lowest_height.min(height), highest_height.max(height))
        });
    let height_range = (highest_height - lowest_height).max(0.001);
    let map_width = map.get_width().max(1) as usize;

    map.get_tiles().iter().enumerate().for_each(|(index, tile)| {
        let x = (index % map_width) as u32;
        let y = height.saturating_sub(1) - (index / map_width) as u32;
        let normalized_height = ((average_tile_height(tile) - lowest_height) / height_range).clamp(0.0, 1.0);
        let walkable = tile.flags.contains(TileFlags::WALKABLE);
        let base = if walkable {
            72.0 + normalized_height * 118.0
        } else {
            26.0 + normalized_height * 52.0
        };

        let pixel = if walkable {
            Rgba([(base * 0.62) as u8, base as u8, (base * 0.66) as u8, 255])
        } else {
            Rgba([(base * 0.45) as u8, (base * 0.50) as u8, (base * 0.58) as u8, 255])
        };

        image.put_pixel(x, y, pixel);
    });

    texture_loader.create_color(&format!("minimap {}", map_name.strip_suffix(".gat").unwrap_or(map_name)), image, false)
}

fn average_tile_height(tile: &Tile) -> f32 {
    (tile.southwest_corner_height + tile.southeast_corner_height + tile.northwest_corner_height + tile.northeast_corner_height) / 4.0
}
