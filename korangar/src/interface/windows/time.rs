use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

const DAWN: f32 = 5.0 * 3600.0;
const NOON: f32 = 12.0 * 3600.0;
const DUSK: f32 = 17.0 * 3600.0;
const MIDNIGHT: f32 = 24.0 * 3600.0;

pub struct TimeWindow;

impl CustomWindow<ClientState> for TimeWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Time)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Time",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                button! {
                    text: "Set dawn",
                    event: InputEvent::SetTime { day_seconds: DAWN },
                },
                button! {
                    text: "Set noon",
                    event: InputEvent::SetTime { day_seconds: NOON },
                },
                button! {
                    text: "Set dusk",
                    event: InputEvent::SetTime { day_seconds: DUSK },
                },
                button! {
                    text: "Set midnight",
                    event: InputEvent::SetTime { day_seconds: MIDNIGHT },
                },
            ),
        }
    }
}
