use korangar_interface::prelude::window;
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use rust_state::Context;

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

#[derive(Default)]
pub struct RespawnWindow;

impl CustomWindow<ClientState> for RespawnWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Respawn)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        window! {
            title: "Respawn Menu",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            elements: (
                button! {
                    text: "Respawn",
                    event: UserEvent::Respawn,
                },
                button! {
                    text: "Disconnect",
                    event: UserEvent::LogOut,
                },
            ),
        }
    }
}
