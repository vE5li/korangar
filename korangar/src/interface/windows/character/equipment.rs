use derive_new::new;
use korangar_interface::elements::ElementWrap;
use korangar_interface::size_bound;
use korangar_interface::state::PlainRemote;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_networking::InventoryItem;

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::EquipmentContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::loaders::ResourceMetadata;

#[derive(new)]
pub struct EquipmentWindow {
    items: PlainRemote<Vec<InventoryItem<ResourceMetadata>>>,
}

impl EquipmentWindow {
    pub const WINDOW_CLASS: &'static str = "equipment";
}

impl PrototypeWindow<InterfaceSettings> for EquipmentWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![EquipmentContainer::new(self.items.clone()).wrap()];

        WindowBuilder::new()
            .with_title("Equipment".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(150 > 200 < 300, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
