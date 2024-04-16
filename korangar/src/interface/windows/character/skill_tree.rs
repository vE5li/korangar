use korangar_interface::elements::ElementWrap;
use korangar_interface::size_bound;
use korangar_interface::state::PlainRemote;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::SkillTreeContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::inventory::Skill;

pub struct SkillTreeWindow {
    skills: PlainRemote<Vec<Skill>>,
}

impl SkillTreeWindow {
    pub fn new(skills: PlainRemote<Vec<Skill>>) -> Self {
        Self { skills }
    }
}

impl SkillTreeWindow {
    pub const WINDOW_CLASS: &'static str = "skill_tree";
}

impl PrototypeWindow<InterfaceSettings> for SkillTreeWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![SkillTreeContainer::new(self.skills.clone()).wrap()];

        WindowBuilder::new()
            .with_title("Skill tree".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ? < 80%))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
