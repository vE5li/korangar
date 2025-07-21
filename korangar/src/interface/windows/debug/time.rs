use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use rust_state::Context;

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};

pub struct TimeWindow;

impl CustomWindow<ClientState> for TimeWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Time)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Time",
            class: Self::window_class(),
            theme: ClientThemeType::Game,
            closable: true,
            elements: (
                button! {
                    text: "Set dawn",
                    event: UserEvent::SetDawn,
                },
                button! {
                    text: "Set noon",
                    event: UserEvent::SetNoon,
                },
                button! {
                    text: "Set dusk",
                    event: UserEvent::SetDusk,
                },
                button! {
                    text: "Set midnight",
                    event: UserEvent::SetMidnight,
                },
            ),
        }
    }
}
