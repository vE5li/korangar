use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::StateElement;
use korangar_interface::event::{Event, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::Friend;
use rust_state::{Context, Path, RustState};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

// TODO: These constants are duplicated troughout the code base. Unify this
// somewhere, maybe a `consts.rs` would be a good idea at this point?
const MINIMUM_NAME_LENGTH: usize = 4;
const MAXIMUM_NAME_LENGTH: usize = 24;

/// Internal state of the chat window.
#[derive(Default, RustState, StateElement)]
pub struct FriendListWindowState {
    currently_adding: String,
}

pub struct FriendListWindow<A, B> {
    window_state_path: A,
    friend_list_path: B,
}

impl<A, B> FriendListWindow<A, B> {
    pub fn new(window_state_path: A, friend_list_path: B) -> Self {
        Self {
            window_state_path,
            friend_list_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for FriendListWindow<A, B>
where
    A: Path<ClientState, FriendListWindowState>,
    B: Path<ClientState, Vec<Friend>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::FriendList)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct AddFriendTextBox;

        let add_action = move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
            let character_name = state.get(&self.window_state_path.currently_adding()).clone();

            // TODO: Give some sort of error if the name is too short.
            if character_name.len() >= MINIMUM_NAME_LENGTH {
                state.update_value_with(self.window_state_path.currently_adding(), |input| input.clear());
                queue.queue(InputEvent::AddFriend { character_name });
                queue.queue(Event::Unfocus);
            }
        };

        window! {
            title: "Friend list",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            elements: (
                text_box! {
                    ghost_text: "Add friend by name",
                    state: self.window_state_path.currently_adding(),
                    input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(self.window_state_path.currently_adding(), add_action),
                    focus_id: AddFriendTextBox,
                },
            )
        }
    }
}
