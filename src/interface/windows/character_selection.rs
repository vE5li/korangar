use procedural::*;
use derive_new::new;

use std::rc::Rc;
use std::cell::RefCell;
use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::InterfaceSettings;
use crate::interface::elements::*;
use crate::interface::{ WindowCache, FramedWindow, ElementCell, Size };
use crate::network::CharacterInformation;

#[derive(new)]
pub struct CharacterSelectionWindow {
    characters: Rc<RefCell<Vec<CharacterInformation>>>,
    move_request: Rc<RefCell<Option<usize>>>,
    changed: Rc<RefCell<bool>>,
    slot_count: usize,
    #[new(value = "\"character_selection\".to_string()")]
    window_class: String,
}

impl PrototypeWindow for CharacterSelectionWindow {

    fn window_class(&self) -> Option<&str> {
        Some(&self.window_class)
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = (0..self.slot_count)
            .into_iter()
            .map(|slot| cell!(CharacterPreview::new(self.characters.clone(), self.move_request.clone(), self.changed.clone(), slot)) as ElementCell)
            .collect();

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "Character Selection".to_string(), self.window_class.clone().into(), elements, constraint!(600.0, ?)))
    }
}
