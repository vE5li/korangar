use procedural::*;

use crate::interface::*;
use crate::inventory::Item;

#[derive(new)]
pub struct EquipmentWindow {
    items: TrackedState<Vec<Item>>,
}

impl EquipmentWindow {
    pub const WINDOW_CLASS: &'static str = "equipment";
}

impl PrototypeWindow for EquipmentWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let elements = vec![EquipmentContainer::new(self.items.new_remote()).wrap()];

        WindowBuilder::default()
            .with_title("Equipment".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(150 > 200 < 300, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
