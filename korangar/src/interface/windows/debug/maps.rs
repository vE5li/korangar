use korangar_interface::elements::{ButtonBuilder, ElementWrap};
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use ragnarok_packets::TilePosition;

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

#[derive(Default)]
pub struct MapsWindow;

impl MapsWindow {
    pub const WINDOW_CLASS: &'static str = "maps";
}

impl PrototypeWindow<InterfaceSettings> for MapsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let map_warps = [
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

        let elements = map_warps
            .into_iter()
            .map(|(name, position)| {
                ButtonBuilder::new()
                    .with_text(name.to_owned())
                    .with_event(UserEvent::RequestWarpToMap(format!("{name}.gat"), position))
                    .build()
                    .wrap()
            })
            .collect();

        WindowBuilder::new()
            .with_title("Maps".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
