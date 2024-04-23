use korangar_interface::elements::ElementWrap;
use korangar_interface::size_bound;
use korangar_interface::state::{PlainTrackedState, TrackedState};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use ragnarok_packets::EntityId;

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::{DialogContainer, DialogElement};
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

pub struct DialogWindow {
    elements: PlainTrackedState<Vec<DialogElement>>,
    npc_id: EntityId,
}

impl DialogWindow {
    pub const WINDOW_CLASS: &'static str = "dialog";

    pub fn new(text: String, npc_id: EntityId) -> (Self, PlainTrackedState<Vec<DialogElement>>) {
        let elements = PlainTrackedState::new(vec![DialogElement::Text(text)]);

        let dialog_window = Self {
            elements: elements.clone(),
            npc_id,
        };

        (dialog_window, elements)
    }
}

impl PrototypeWindow<InterfaceSettings> for DialogWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![DialogContainer::new(self.elements.new_remote(), self.npc_id).wrap()];

        WindowBuilder::new()
            .with_title("Dialog".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
