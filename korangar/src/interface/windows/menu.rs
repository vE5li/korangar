use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::theme::{DebugButtonThemePathExt, InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ClientState, client_theme};

#[derive(Default)]
pub struct MenuWindow;

impl CustomWindow<ClientState> for MenuWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Menu)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Menu",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            elements: (
                button! {
                    text: "Graphics settings",
                    event: InputEvent::ToggleGraphicsSettingsWindow,
                },
                button! {
                    text: "Audio settings",
                    event: InputEvent::ToggleAudioSettingsWindow,
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Render options",
                    tooltip: "Special render options (only available in debug mode)",
                    event: InputEvent::ToggleRenderOptionsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Map inspector",
                    tooltip: "Inspect the raw map data (only available in debug mode)",
                    event: InputEvent::OpenMapDataWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Client state inspector",
                    tooltip: "Inspect and modify the internal client state (only available in debug mode)",
                    event: InputEvent::ToggleClientStateInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Maps",
                    tooltip: "List of maps used for testing (only available in debug mode)",
                    event: InputEvent::ToggleMapsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Commands",
                    tooltip: "List of commands used for testing (only available in debug mode)",
                    event: InputEvent::ToggleCommandsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Time",
                    tooltip: "Time control (only available in debug mode)",
                    event: InputEvent::ToggleTimeWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Theme inspector",
                    tooltip: "Inspect and edit the theme (only available in debug mode)",
                    event: InputEvent::OpenThemeInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Profiler",
                    tooltip: "Profile the client (only available in debug mode)",
                    event: InputEvent::ToggleProfilerWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Packet inspector",
                    tooltip: "Inspect all incoming and outgoing packets (only available in debug mode)",
                    event: InputEvent::TogglePacketInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                button! {
                    text: "Log out",
                    event: InputEvent::LogOut,
                },
                button! {
                    text: "Exit",
                    event: InputEvent::Exit,
                },
            ),
        }
    }
}
