use korangar_interface::prelude::window;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::Context;

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::state::{ClientState, ClientThemeType};

#[derive(Default)]
pub struct RespawnWindow;

impl RespawnWindow {
    pub const WINDOW_CLASS: &'static str = "respawn";
}

impl CustomWindow<ClientState> for RespawnWindow {
    fn window_class() -> Option<&'static str> {
        Some(Self::WINDOW_CLASS)
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        window! {
            title: "Respawn Menu",
            window_id: 0,
            theme: ClientThemeType::Game,
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
