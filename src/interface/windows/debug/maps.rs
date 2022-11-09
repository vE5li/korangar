use cgmath::Vector2;
use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;

#[derive(Default)]
pub struct MapsWindow {}

impl MapsWindow {
    pub const WINDOW_CLASS: &'static str = "maps";
}

impl PrototypeWindow for MapsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
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
                Button::default()
                    .with_dynamic_text(name.to_owned())
                    .with_event(UserEvent::RequestWarpToMap(format!("{}.gat", name), position))
                    .wrap()
            })
            .collect();

        WindowBuilder::default()
            .with_title("Maps".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(200 > 250 < 300, ? < 80%))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
