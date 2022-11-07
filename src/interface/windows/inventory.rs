use procedural::*;

use crate::interface::*;
use crate::inventory::Item;

#[derive(new)]
pub struct InventoryWindow {
    items: TrackedState<Vec<Item>>,
}

impl InventoryWindow {
    pub const WINDOW_CLASS: &'static str = "invetory";
}

impl PrototypeWindow for InventoryWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        let elements = vec![InventoryContainer::new(self.items.new_remote()).wrap()];

        WindowBuilder::default()
            .with_title("Inventory".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(300 > 400 < 500, ? < 80%))
            .with_elements(elements)
            .closeable()
            .build(window_cache, interface_settings, avalible_space)
    }
}
