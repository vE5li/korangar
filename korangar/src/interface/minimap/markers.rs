use ragnarok_packets::{Direction, TilePosition};

use crate::graphics::Color;
use crate::world::{Entity, EntityType};

pub struct MinimapMarker {
    pub tile_position: TilePosition,
    pub symbol: &'static str,
    pub color: Color,
    pub size: f32,
    pub font_size: f32,
    pub is_player: bool,
    pub player_direction: Option<Direction>,
}

pub fn collect_minimap_markers(entities: &[Entity], player_entity: Option<&Entity>, base_size: f32) -> Vec<MinimapMarker> {
    let player_entity_id = player_entity.map(Entity::get_entity_id);
    let player_direction = player_entity.map(Entity::get_direction).unwrap_or(Direction::South);

    entities
        .iter()
        .filter_map(|entity| {
            let is_player = player_entity_id.is_some_and(|player_entity_id| player_entity_id == entity.get_entity_id());
            let entity_type = entity.get_entity_type();

            if entity_type == EntityType::Hidden {
                return None;
            }

            let size = if is_player { base_size + 2.0 } else { base_size };
            let symbol = if is_player { "" } else { "●" };
            let font_size = size + if is_player { 8.0 } else { 6.0 };

            Some(MinimapMarker {
                tile_position: entity.get_tile_position(),
                symbol,
                color: marker_color(entity_type, is_player),
                size,
                font_size,
                is_player,
                player_direction: if is_player { Some(player_direction) } else { None },
            })
        })
        .collect()
}

fn marker_color(entity_type: EntityType, is_player: bool) -> Color {
    if is_player {
        return Color::rgb_u8(80, 255, 160);
    }

    match entity_type {
        EntityType::Player => Color::rgb_u8(100, 180, 255),
        EntityType::Npc => Color::rgb_u8(255, 200, 80),
        EntityType::Monster => Color::rgb_u8(255, 110, 120),
        EntityType::Warp => Color::rgb_u8(180, 255, 255),
        EntityType::Hidden => Color::rgba_u8(0, 0, 0, 0),
    }
}
