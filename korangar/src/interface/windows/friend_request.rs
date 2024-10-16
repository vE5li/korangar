use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::Friend;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct FriendRequestWindow {
    friend: Friend,
}

impl FriendRequestWindow {
    pub fn new(friend: Friend) -> Self {
        Self { friend }
    }
}

impl CustomWindow<ClientState> for FriendRequestWindow {
    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Friend request",
            class: Some(WindowClass::FriendRequest),
            theme: InterfaceThemeType::Game,
            closable: true,
            elements: (
                text! {
                    text: format!("^000001{}^000000 wants to be friends with you", self.friend.name),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Reject",
                            event: InputEvent::RejectFriendRequest {
                                account_id: self.friend.account_id,
                                character_id: self.friend.character_id,
                            },
                        },
                        button! {
                            text: "Accept",
                            event: InputEvent::AcceptFriendRequest {
                                account_id: self.friend.account_id,
                                character_id: self.friend.character_id,
                            },
                        },
                    ),
                },
            ),
        }
    }
}
