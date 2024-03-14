use derive_new::new;
use korangar_interface::elements::ElementWrap;
use korangar_interface::state::PlainRemote;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_procedural::size_bound;

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::InventoryContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::inventory::Item;

#[derive(new)]
pub struct InventoryWindow {
    items: PlainRemote<Vec<Item>>,
}

impl InventoryWindow {
    pub const WINDOW_CLASS: &'static str = "inventory";
}

impl PrototypeWindow<InterfaceSettings> for InventoryWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![InventoryContainer::new(self.items.clone()).wrap()];

        WindowBuilder::new()
            .with_title("Inventory".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ? < 80%))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
