use procedural::*;

use crate::interface::*;
use crate::inventory::Skill;

#[derive(new)]
pub struct SkillTreeWindow {
    skills: Remote<Vec<Skill>>,
}

impl SkillTreeWindow {
    pub const WINDOW_CLASS: &'static str = "skill_tree";
}

impl PrototypeWindow for SkillTreeWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let elements = vec![SkillTreeContainer::new(self.skills.clone()).wrap()];

        WindowBuilder::new()
            .with_title("Skill tree".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ? < 80%))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
