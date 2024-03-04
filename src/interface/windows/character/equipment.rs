use procedural::*;

use crate::interface::*;
use crate::inventory::Item;

#[derive(new)]
pub struct EquipmentWindow {
    items: Remote<Vec<Item>>,
}

impl EquipmentWindow {
    pub const WINDOW_CLASS: &'static str = "equipment";
}

impl PrototypeWindow for EquipmentWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let elements = vec![EquipmentContainer::new(self.items.clone()).wrap()];

        WindowBuilder::new()
            .with_title("Equipment".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(150 > 200 < 300, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
