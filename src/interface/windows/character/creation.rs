use std::cell::RefCell;
use std::rc::Rc;

use derive_new::new;
use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;

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

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let name = Rc::new(RefCell::new(String::new()));

        let selector = {
            let name = name.clone();
            move || name.borrow().len() >= MINIMUM_NAME_LENGTH
        };

        let action = {
            let slot = self.slot;
            let name = name.clone();

            move || vec![ClickAction::Event(UserEvent::CreateCharacter(slot, name.borrow().clone()))]
        };

        let input_action = Box::new(move || vec![ClickAction::FocusNext(FocusMode::FocusNext)]);

        let elements = vec![
            InputField::<MAXIMUM_NAME_LENGTH>::new(name, "character name", input_action, dimension_bound!(100%)).wrap(),
            ButtonBuilder::new()
                .with_text("done")
                .with_disabled_selector(selector)
                .with_event(Box::new(action))
                .with_width_bound(dimension_bound!(50%))
                .build()
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Create Character".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .closable()
            .with_theme_kind(ThemeKind::Menu)
            .build(window_cache, interface_settings, available_space)
    }
}
