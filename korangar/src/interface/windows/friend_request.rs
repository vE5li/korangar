use derive_new::new;
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use ragnarok_packets::Friend;

use crate::graphics::Color;
use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
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
                            event: UserEvent::RejectFriendRequest {
                                account_id: self.friend.account_id,
                                character_id: self.friend.character_id,
                            },
                        },
                        button! {
                            text: "Accept",
                            event: UserEvent::AcceptFriendRequest {
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
