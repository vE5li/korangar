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

    fn to_window(
        &self,
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
    ) -> Box<dyn Window + 'static> {
        let elements = vec![EquipmentContainer::new(self.items.new_remote()).wrap()];

        Box::from(FramedWindow::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Equipment".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(150 > 200 < 300, ?),
            true,
        ))
    }
}
