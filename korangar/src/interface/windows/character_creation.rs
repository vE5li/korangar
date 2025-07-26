use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::event::EventQueue;
use korangar_interface::window::{CustomWindow, WindowTrait};
use rust_state::{Context, Path};

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};

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

        window! {
            title: "Create Character",
            class: Self::window_class(),
            theme: ClientThemeType::Menu,
            closable: true,
            elements: (
                text_box! {
                    text: "Character name",
                    state: self.path,
                    input_handler: DefaultHandler(self.path),
                },
                button! {
                    text: "Create",
                    event: move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
                        let name = state.get(&self.path).clone();
                        queue.queue(UserEvent::CreateCharacter { slot: self.slot, name });
                    }
                }
            ),
        }
    }
}
