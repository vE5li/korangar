use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::TilePosition;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct MapsWindow;

impl CustomWindow<ClientState> for MapsWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Maps)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        const MAP_COUNT: usize = 23;
        const MAP_WARPS: [(&str, TilePosition); MAP_COUNT] = [
            ("geffen", TilePosition { x: 119, y: 59 }),
            ("alberta", TilePosition { x: 28, y: 234 }),
            ("aldebaran", TilePosition { x: 140, y: 131 }),
            ("amatsu", TilePosition { x: 198, y: 84 }),
            ("ayothaya", TilePosition { x: 208, y: 166 }),
            ("prontera", TilePosition { x: 155, y: 183 }),
            ("brasilis", TilePosition { x: 196, y: 217 }),
            ("einbech", TilePosition { x: 63, y: 35 }),
            ("einbroch", TilePosition { x: 64, y: 200 }),
            ("dicastes01", TilePosition { x: 198, y: 187 }),
            ("gonryun", TilePosition { x: 160, y: 120 }),
            ("hugel", TilePosition { x: 96, y: 145 }),
            ("izlude", TilePosition { x: 128, y: 146 }),
            ("jawaii", TilePosition { x: 251, y: 132 }),
            ("lasagna", TilePosition { x: 193, y: 182 }),
            ("lighthalzen", TilePosition { x: 158, y: 92 }),
            ("louyang", TilePosition { x: 217, y: 100 }),
            ("xmas", TilePosition { x: 147, y: 134 }),
            ("c_tower1", TilePosition { x: 235, y: 218 }),
            ("ama_dun01", TilePosition { x: 54, y: 107 }),
            ("umbala", TilePosition { x: 97, y: 153 }),
            ("rachel", TilePosition { x: 120, y: 120 }),
            ("mid_camp", TilePosition { x: 180, y: 240 }),
        ];

        window! {
            title: "Maps",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: std::array::from_fn::<_, MAP_COUNT, _>(|index| {
                let warp = MAP_WARPS[index];

                button! {
                    text: warp.0,
                    tooltip: format!("Map: {}\nCoordinates: {}, {}", warp.0, warp.1.x, warp.1.y),
                    event: InputEvent::WarpToMap {
                        map_name: format!("{}.gat", warp.0),
                        position: warp.1,
                    },
                }
            }),
        }
    }
}
