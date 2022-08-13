use procedural::*;
use std::rc::Rc;
use std::cell::RefCell;

use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::{InterfaceSettings, DialogElement};
use crate::interface::elements::*;
use crate::interface::{ WindowCache, FramedWindow, ElementCell, Size };

pub struct DialogWindow {
    elements: Rc<RefCell<Vec<DialogElement>>>,
    changed: Rc<RefCell<bool>>,
    npc_id: u32,
}

impl DialogWindow {

    pub const WINDOW_CLASS: &'static str = "dialog";

    pub fn new(text: String, npc_id: u32) -> (Self, Rc<RefCell<Vec<DialogElement>>>, Rc<RefCell<bool>>) {

        let elements = Rc::new(RefCell::new(vec![DialogElement::Text(text)]));
        let changed = Rc::new(RefCell::new(false));

        let dialog_window = Self {
            elements: elements.clone(),
            changed: changed.clone(),
            npc_id,
        };

        (dialog_window, elements, changed)
    }
}

impl PrototypeWindow for DialogWindow {

    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            cell!(DialogContainer::new(self.elements.clone(), self.changed.clone(), self.npc_id)),
        ];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "Dialog".to_string(), Self::WINDOW_CLASS.to_string().into(), elements, constraint!(300 > 400 < 500, ?)))
    }
}
