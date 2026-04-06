use korangar_interface::layout::area::Area;
use ragnarok_packets::TilePosition;

pub struct MinimapProjection {
    texture_area: Area,
    map_width: f32,
    map_height: f32,
}

impl MinimapProjection {
    pub fn new(view_area: Area, map_width: u16, map_height: u16, zoom: f32, focus_position: Option<TilePosition>) -> Self {
        if map_width == 0 || map_height == 0 {
            return Self {
                texture_area: view_area,
                map_width: 1.0,
                map_height: 1.0,
            };
        }

        let map_width = map_width as f32;
        let map_height = map_height as f32;
        let scale = (view_area.width / map_width).min(view_area.height / map_height) * zoom.max(1.0);
        let texture_width = map_width * scale;
        let texture_height = map_height * scale;
        let centered_area = view_area.interior(
            texture_width,
            texture_height,
            korangar_interface::prelude::HorizontalAlignment::Center {
                offset: 0.0,
                border: 0.0,
            },
            korangar_interface::prelude::VerticalAlignment::Center { offset: 0.0 },
        );

        let texture_area = focus_position.map_or(centered_area, |focus_position| {
            let (normalized_x, normalized_y) = Self::normalized_tile_position(map_width, map_height, focus_position);
            let desired_left = view_area.left + view_area.width / 2.0 - normalized_x * texture_width;
            let desired_top = view_area.top + view_area.height / 2.0 - normalized_y * texture_height;
            let left = if texture_width > view_area.width {
                desired_left.clamp(view_area.left + view_area.width - texture_width, view_area.left)
            } else {
                centered_area.left
            };
            let top = if texture_height > view_area.height {
                desired_top.clamp(view_area.top + view_area.height - texture_height, view_area.top)
            } else {
                centered_area.top
            };

            Area {
                left,
                top,
                width: texture_width,
                height: texture_height,
            }
        });

        Self {
            texture_area,
            map_width,
            map_height,
        }
    }

    pub fn texture_area(&self) -> Area {
        self.texture_area
    }

    pub fn marker_area(&self, position: TilePosition, size: f32) -> Area {
        let (normalized_x, normalized_y) = Self::normalized_tile_position(self.map_width, self.map_height, position);
        let left = self.texture_area.left + normalized_x * self.texture_area.width - size / 2.0;
        let top = self.texture_area.top + normalized_y * self.texture_area.height - size / 2.0;

        Area {
            left,
            top,
            width: size,
            height: size,
        }
    }

    fn normalized_tile_position(map_width: f32, map_height: f32, position: TilePosition) -> (f32, f32) {
        let x = (position.x as f32 + 0.5) / map_width.max(1.0);
        let y = ((map_height - position.y as f32) - 0.5) / map_height.max(1.0);

        (x, y)
    }
}
