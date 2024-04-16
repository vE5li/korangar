use derive_new::new;
use korangar_interface::elements::{ButtonBuilder, ElementWrap, FocusMode, InputFieldBuilder};
use korangar_interface::event::ClickAction;
use korangar_interface::state::{PlainTrackedState, TrackedState, TrackedStateClone};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::theme::InterfaceThemeKind;
use crate::interface::windows::WindowCache;

const MINIMUM_NAME_LENGTH: usize = 4;
const MAXIMUM_NAME_LENGTH: usize = 24;

#[derive(new)]
pub struct CharacterCreationWindow {
    slot: usize,
}

impl CharacterCreationWindow {
    pub const WINDOW_CLASS: &'static str = "character_creation";
}

impl PrototypeWindow<InterfaceSettings> for CharacterCreationWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let name = PlainTrackedState::<String>::default();

        let selector = {
            let name = name.clone();
            move || name.get().len() >= MINIMUM_NAME_LENGTH
        };

        let action = {
            let slot = self.slot;
            let name = name.clone();

            move || vec![ClickAction::Custom(UserEvent::CreateCharacter(slot, name.cloned()))]
        };

        let input_action = Box::new(move || vec![ClickAction::FocusNext(FocusMode::FocusNext)]);

        let elements = vec![
            InputFieldBuilder::new()
                .with_state(name)
                .with_ghost_text("Character name")
                .with_enter_action(input_action)
                .with_length(MAXIMUM_NAME_LENGTH)
                .build()
                .wrap(),
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
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .with_theme_kind(InterfaceThemeKind::Menu)
            .build(window_cache, application, available_space)
    }
}
