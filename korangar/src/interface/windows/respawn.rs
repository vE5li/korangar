use korangar_interface::window::{CustomWindow, WindowTrait};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

#[derive(Default)]
pub struct RespawnWindow;

impl CustomWindow<ClientState> for RespawnWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Respawn)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Respawn Menu",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            elements: (
                button! {
                    text: "Respawn",
                    event: InputEvent::Respawn,
                },
                button! {
                    text: "Disconnect",
                    event: InputEvent::LogOut,
                },
            ),
        }
    }
}
