use crate::interface::*;

pub struct DialogWindow {
    elements: TrackedState<Vec<DialogElement>>,
    npc_id: EntityId,
}

impl DialogWindow {
    pub const WINDOW_CLASS: &'static str = "dialog";

    pub fn new(text: String, npc_id: EntityId) -> (Self, TrackedState<Vec<DialogElement>>) {
        let elements = TrackedState::new(vec![DialogElement::Text(text)]);

        let dialog_window = Self {
            elements: elements.clone(),
            npc_id,
        };

        (dialog_window, elements)
    }
}

impl PrototypeWindow for DialogWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let elements = vec![DialogContainer::new(self.elements.new_remote(), self.npc_id).wrap()];

        WindowBuilder::default()
            .with_title("Dialog".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .build(window_cache, interface_settings, available_space)
    }
}
