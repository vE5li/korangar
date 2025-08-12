use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::theme::InterfaceThemeType;
use crate::state::translation::TranslationPathExt;
use crate::state::{ClientState, ClientStatePathExt, client_state};

#[derive(Default)]
pub struct RespawnWindow;

impl CustomWindow<ClientState> for RespawnWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Respawn)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().translation().respawn_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            elements: (
                button! {
                    text: client_state().translation().respawn_button_text(),
                    event: InputEvent::Respawn,
                },
                button! {
                    text: client_state().translation().disconnect_button_text(),
                    event: InputEvent::LogOut,
                },
            ),
        }
    }
}
