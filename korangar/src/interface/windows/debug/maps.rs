use cgmath::Vector2;
use korangar_interface::elements::{ButtonBuilder, ElementWrap};
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};

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
            ("geffen", Vector2::new(119, 59)),
            ("alberta", Vector2::new(28, 234)),
            ("aldebaran", Vector2::new(140, 131)),
            ("amatsu", Vector2::new(198, 84)),
            ("ayothaya", Vector2::new(208, 166)),
            ("prontera", Vector2::new(155, 183)),
            ("brasilis", Vector2::new(196, 217)),
            ("einbech", Vector2::new(63, 35)),
            ("einbroch", Vector2::new(64, 200)),
            ("dicastes01", Vector2::new(198, 187)),
            ("gonryun", Vector2::new(160, 120)),
            ("hugel", Vector2::new(96, 145)),
            ("izlude", Vector2::new(128, 146)),
            ("jawaii", Vector2::new(251, 132)),
            ("lasagna", Vector2::new(193, 182)),
            ("lighthalzen", Vector2::new(158, 92)),
            ("louyang", Vector2::new(217, 100)),
            ("xmas", Vector2::new(147, 134)),
            ("c_tower1", Vector2::new(235, 218)),
            ("ama_dun01", Vector2::new(54, 107)),
            ("umbala", Vector2::new(97, 153)),
            ("rachel", Vector2::new(120, 120)),
            ("mid_camp", Vector2::new(180, 240)),
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
