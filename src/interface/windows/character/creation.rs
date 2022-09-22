use std::cell::RefCell;
use std::rc::Rc;

use derive_new::new;
use procedural::*;

use crate::input::UserEvent;
use crate::interface::{ElementCell, FramedWindow, InterfaceSettings, PrototypeWindow, Size, Window, WindowCache, *};

const MINIMUM_NAME_LENGTH: usize = 4;
const MAXIMUM_NAME_LENGTH: usize = 24;

#[derive(new)]
pub struct CharacterCreationWindow {
    slot: usize,
}

impl CharacterCreationWindow {

    pub const WINDOW_CLASS: &'static str = "character_creation";
}

impl PrototypeWindow for CharacterCreationWindow {

    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
    ) -> Box<dyn Window + 'static> {

        let name = Rc::new(RefCell::new(String::new()));

        let selector = {

            let name = name.clone();

            move || name.borrow().len() >= MINIMUM_NAME_LENGTH
        };

        let action = {

            let slot = self.slot;
            let name = name.clone();

            move || Some(ClickAction::Event(UserEvent::CreateCharacter(slot, name.borrow().clone())))
        };

        let input_action = Box::new(move || Some(ClickAction::FocusNext(FocusMode::FocusNext)));

        let elements: Vec<ElementCell> = vec![
            cell!(InputField::<MAXIMUM_NAME_LENGTH>::new(
                name,
                "character name",
                input_action,
                dimension!(100%)
            )),
            Button::default()
                .with_static_text("done")
                .with_disabled_selector(selector)
                .with_action_closure(action)
                .with_width(dimension!(50%))
                .wrap(),
        ];

        Box::from(FramedWindow::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Create Character".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(200 > 250 < 300, ? < 80%),
        ))
    }
}
