use derive_new::new;
use korangar_interface::elements::ElementWrap;
use korangar_interface::size_bound;
use korangar_interface::state::PlainRemote;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::HotbarContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::inventory::Skill;

#[derive(new)]
pub struct HotbarWindow {
    skills: PlainRemote<[Option<Skill>; 10]>,
}

impl HotbarWindow {
    pub const WINDOW_CLASS: &'static str = "hotbar";
}

impl PrototypeWindow<InterfaceSettings> for HotbarWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![HotbarContainer::new(self.skills.clone()).wrap()];

        WindowBuilder::new()
            .with_title("Hotbar".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ?))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
