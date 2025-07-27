use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::event::EventQueue;
use korangar_interface::window::{CustomWindow, WindowTrait};
use rust_state::{Context, Path};

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

const MINIMUM_NAME_LENGTH: usize = 4;
const MAXIMUM_NAME_LENGTH: usize = 24;

pub struct CharacterCreationWindow<P> {
    path: P,
    slot: usize,
}

impl<P> CharacterCreationWindow<P> {
    pub fn new(path: P, slot: usize) -> Self {
        Self { path, slot }
    }
}

impl<P> CustomWindow<ClientState> for CharacterCreationWindow<P>
where
    P: Path<ClientState, String>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterCreation)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct CharacterName;

        let create_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&self.path).clone();

            // TODO: Give some sort of error if the name is too short.
            if name.len() >= MINIMUM_NAME_LENGTH {
                queue.queue(UserEvent::CreateCharacter { slot: self.slot, name });
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
                    state: self.path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.path, create_action),
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
