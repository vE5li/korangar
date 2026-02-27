use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

const MINIMUM_NAME_LENGTH: usize = 4;
const MAXIMUM_NAME_LENGTH: usize = 24;

pub struct CharacterCreationWindow<A> {
    character_name_path: A,
    slot: usize,
}

impl<A> CharacterCreationWindow<A> {
    pub fn new(character_name_path: A, slot: usize) -> Self {
        Self { character_name_path, slot }
    }
}

impl<A> CustomWindow<ClientState> for CharacterCreationWindow<A>
where
    A: Path<ClientState, String>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterCreation)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct CharacterName;

        let disabled = ComputedSelector::new_default(move |state: &ClientState| {
            self.character_name_path.follow_safe(state).len() < MINIMUM_NAME_LENGTH
        });

        let create_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&self.character_name_path).clone();
            queue.queue(InputEvent::CreateCharacter { slot: self.slot, name });
        };

        window! {
            title: client_state().localization().create_character_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            closable: true,
            elements: (
                text_box! {
                    ghost_text: client_state().localization().character_name_text(),
                    state: self.character_name_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.character_name_path, create_action),
                    focus_id: CharacterName,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                button! {
                    text: client_state().localization().create_character_button_text(),
                    disabled,
                    disabled_tooltip: client_state().localization().create_character_button_tooltip(),
                    event: create_action,
                }
            ),
        }
    }
}
