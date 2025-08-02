use korangar_interface::window::{CustomWindow, WindowTrait};
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
    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Friend request",
            class: Some(WindowClass::FriendRequest),
            theme: InterfaceThemeType::Menu,
            closable: true,
            elements: (
                text! {
                    text: format!("^ffaa00{}^000000 wants to be friends with you", self.friend.name),
                },
                split! {
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
