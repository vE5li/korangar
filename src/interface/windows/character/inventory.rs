use procedural::*;

use crate::interface::*;
use crate::inventory::Item;

#[derive(new)]
pub struct InventoryWindow {
    items: Remote<Vec<Item>>,
}

impl InventoryWindow {
    pub const WINDOW_CLASS: &'static str = "inventory";
}

impl PrototypeWindow for InventoryWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let elements = vec![InventoryContainer::new(self.items.clone()).wrap()];

        WindowBuilder::default()
            .with_title("Inventory".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(300 > 400 < 500, ? < 80%))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
