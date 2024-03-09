use procedural::size_bound;

use crate::interface::*;
use crate::inventory::Skill;

#[derive(new)]
pub struct HotbarWindow {
    skills: Remote<[Option<Skill>; 10]>,
}

impl HotbarWindow {
    pub const WINDOW_CLASS: &'static str = "hotbar";
}

impl PrototypeWindow for HotbarWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let elements = vec![HotbarContainer::new(self.skills.clone()).wrap()];

        WindowBuilder::new()
            .with_title("Hotbar".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ?))
            .with_elements(elements)
            .build(window_cache, interface_settings, available_space)
    }
}
