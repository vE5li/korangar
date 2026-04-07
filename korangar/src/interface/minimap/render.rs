use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use rust_state::{Path, State};

use super::markers::collect_minimap_markers;
use super::projection::MinimapProjection;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::{this_entity, ClientState, MinimapState, ClientStatePathExt};
use crate::world::Entity;

pub struct MinimapLayoutInfo {
    map_area: Area,
    status_area: Area,
}

pub struct MinimapView<A, B> {
    minimap_path: A,
    entities_path: B,
    status_text: String,
}

impl<A, B> MinimapView<A, B> {
    pub fn new(minimap_path: A, entities_path: B) -> Self {
        Self {
            minimap_path,
            entities_path,
            status_text: String::new(),
        }
    }
}

impl<A, B> Element<ClientState> for MinimapView<A, B>
where
    A: Path<ClientState, MinimapState>,
    B: Path<ClientState, Vec<Entity>>,
{
    type LayoutInfo = MinimapLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let map_area = resolver.with_height(192.0);
            let status_area = resolver.with_height(22.0);
            let minimap = state.get(&self.minimap_path);
            let player_position = state.try_get(&this_entity()).map(Entity::get_tile_position);

            self.status_text = if minimap.map_name.is_empty() {
                "Loading map".to_owned()
            } else if let Some(position) = player_position {
                format!("{} ({}, {})", minimap.map_name, position.x, position.y)
            } else {
                minimap.map_name.clone()
            };

            MinimapLayoutInfo { map_area, status_area }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let minimap = state.get(&self.minimap_path);
        let player_position = state.try_get(&this_entity()).map(Entity::get_tile_position);

        layout.add_rectangle(
            layout_info.map_area,
            CornerDiameter::uniform(6.0),
            Color::rgb_u8(24, 26, 32),
            Color::rgba_u8(0, 0, 0, 110),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        if let Some(texture) = &minimap.texture {
            let projection = MinimapProjection::new(layout_info.map_area, minimap.width, minimap.height, minimap.zoom, player_position);

            layout.add_texture(projection.texture_area(), texture.clone(), Color::WHITE, false);

            let entities = state.get(&self.entities_path);
            let player_path = this_entity();
            let player_entity = state.try_get(&player_path);
            let marker_size = (projection.texture_area().width.min(projection.texture_area().height) / 40.0).clamp(4.0, 8.0);
            
            // To get camera info, we need a way to pass it here or just fallback to 0.0 rotation.
            // Ideally, the camera should be accessible via state, but if not we can add it to ClientState
            // For now, let's use the camera view angle from the layout or client state
            let view_angle = *state.get(&crate::state::client_state().camera_view_angle());

            collect_minimap_markers(entities, player_entity, marker_size)
                .into_iter()
                .for_each(|marker| {
                    if marker.is_player {
                        if let Some(arrow_texture) = &minimap.arrow_texture {
                            // Map the 8-way walking direction to an angle in radians
                            let direction_angle = match marker.player_direction {
                                Some(ragnarok_packets::Direction::West) => 0.0,
                                Some(ragnarok_packets::Direction::NorthWest) => std::f32::consts::PI / 4.0,
                                Some(ragnarok_packets::Direction::North) => std::f32::consts::PI / 2.0,
                                Some(ragnarok_packets::Direction::NorthEast) => 3.0 * std::f32::consts::PI / 4.0,
                                Some(ragnarok_packets::Direction::East) => std::f32::consts::PI,
                                Some(ragnarok_packets::Direction::SouthEast) => 5.0 * std::f32::consts::PI / 4.0,
                                Some(ragnarok_packets::Direction::South) => 3.0 * std::f32::consts::PI / 2.0,
                                Some(ragnarok_packets::Direction::SouthWest) => 7.0 * std::f32::consts::PI / 4.0,
                                None => 0.0,
                            };

                            // Since the minimap renders top-down, we add the camera's view angle 
                            // to the character's walking direction to ensure the arrow points relative to what you see.
                            let total_rotation = direction_angle - view_angle;

                            // We negate the angle because the SDF shader needs negative rotation to turn clockwise on screen,
                            // matching the minimap's coordinate system.
                            let rotation = -total_rotation;
                            
                            let mut area = projection.marker_area(marker.tile_position, marker.size);
                            // Make the arrow significantly larger and sharper to be clearly visible
                            area.width *= 2.0;
                            area.height *= 2.0;
                            area.left -= marker.size / 2.0;
                            area.top -= marker.size / 2.0;

                            // Use an explicit sharp color like bright yellow or cyan for the player marker instead of the generic green/white
                            let player_marker_color = Color::rgb_u8(255, 215, 0); // Bright Yellow

                            // The arrow_right.png points exactly East (0 radians), so we don't need
                            // an additional rotation offset like we might have needed with arrow_left.png.
                            layout.add_rotated_sdf(area, arrow_texture.clone(), player_marker_color, rotation);
                            return;
                        }
                    }

                    layout.add_text(
                        projection.marker_area(marker.tile_position, marker.size),
                        marker.symbol,
                        FontSize(marker.font_size),
                        marker.color,
                        Color::BLACK,
                        HorizontalAlignment::Center {
                            offset: 0.0,
                            border: 0.0,
                        },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );
                });
        }

        layout.add_text(
            layout_info.status_area,
            &self.status_text,
            FontSize(14.0),
            Color::monochrome_u8(225),
            Color::rgb_u8(255, 180, 80),
            HorizontalAlignment::Center {
                offset: 0.0,
                border: 4.0,
            },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );
    }
}
