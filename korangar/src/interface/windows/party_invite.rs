use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::PartyId;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct PartyInviteWindow {
    party_id: PartyId,
    party_name: String,
}

impl PartyInviteWindow {
    pub fn new(party_id: PartyId, party_name: String) -> Self {
        Self { party_id, party_name }
    }
}

impl CustomWindow<ClientState> for PartyInviteWindow {
    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Party invite",
            class: Some(WindowClass::PartyInvite),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: format!("You have been invited to join ^000001{}^000000", self.party_name),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Reject",
                            event: InputEvent::RejectPartyInvite {
                                party_id: self.party_id,
                            },
                        },
                        button! {
                            text: "Accept",
                            event: InputEvent::AcceptPartyInvite {
                                party_id: self.party_id,
                            },
                        },
                    ),
                },
            ),
        }
    }
}
