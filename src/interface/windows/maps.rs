use derive_new::new;

use crate::input::UserEvent;
use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::InterfaceSettings;
use crate::interface::elements::*;
use crate::interface::{ WindowCache, FramedWindow, ElementCell, Size };
use crate::types::maths::Vector2;

#[derive(new)]
pub struct MapsWindow {
    #[new(value = "\"maps\".to_string()")]
    window_class: String,
}

impl PrototypeWindow for MapsWindow {

    fn window_class(&self) -> Option<&str> {
        Some(&self.window_class)
    } 

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let map_warps = [
            ("geffen", Vector2::new(119, 59)),
            ("alberta",Vector2::new(28,234)),
            ("aldebaran",Vector2::new(140,131)),
            ("amatsu",Vector2::new(198,84)),
            ("ayothaya",Vector2::new(208,166)),
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

        ];

        let elements = map_warps 
            .into_iter()
            .map(|(name, position)| cell!(EventButton::new(name.to_string(), UserEvent::RequestWarpToMap(format!("{}.gat", name), position))) as ElementCell)
            .collect();
        
        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "maps".to_string(), self.window_class.clone().into(), elements, constraint!(200.0 > 250.0 < 300.0, ? < 80.0%)))
    }
}
