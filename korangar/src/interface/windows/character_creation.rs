use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::event::EventQueue;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Context, Path};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

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

        let create_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&self.character_name_path).clone();

            // TODO: Give some sort of error if the name is too short.
            if name.len() >= MINIMUM_NAME_LENGTH {
                queue.queue(InputEvent::CreateCharacter { slot: self.slot, name });
            }
        };

        window! {
            title: "Create Character",
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            closable: true,
            elements: (
                text_box! {
                    ghost_text: "Character name",
                    state: self.character_name_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.character_name_path, create_action),
                    focus_id: CharacterName,
                },
                button! {
                    text: "Create",
                    // TODO: Disable if the name is too short.
                    // disabled: selector,
                    event: create_action,
                }
            ),
        }
    }
}
