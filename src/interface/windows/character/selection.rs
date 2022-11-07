use derive_new::new;
use procedural::*;

use crate::interface::*;
use crate::network::CharacterInformation;

#[derive(new)]
pub struct CharacterSelectionWindow {
    characters: TrackedState<Vec<CharacterInformation>>,
    move_request: TrackedState<Option<usize>>,
    slot_count: usize,
}

impl CharacterSelectionWindow {
    pub const WINDOW_CLASS: &'static str = "character_selection";
}

impl PrototypeWindow for CharacterSelectionWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        let elements: Vec<ElementCell> = (0..self.slot_count)
            .into_iter()
            .map(|slot| {
                cell!(CharacterPreview::new(
                    self.characters.new_remote(),
                    self.move_request.new_remote(),
                    slot
                )) as ElementCell
            })
            .collect();

        Window::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Character Selection".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(600, ?),
            false,
        )
    }
}
