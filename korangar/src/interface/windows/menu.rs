use korangar_interface::window::{CustomWindow, WindowTrait};

use crate::input::UserEvent;
use crate::interface::windows::WindowClass;
use crate::state::theme::{DebugButtonThemePathExt, InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ClientState, client_theme};

#[derive(Default)]
pub struct MenuWindow;

impl CustomWindow<ClientState> for MenuWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Menu)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Menu",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            elements: (
                button! {
                    text: "Graphics settings",
                    event: UserEvent::OpenGraphicsSettingsWindow,
                },
                button! {
                    text: "Audio settings",
                    event: UserEvent::OpenAudioSettingsWindow,
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Render options",
                    tooltip: "Special render options (only available in debug mode)",
                    event: UserEvent::OpenRenderOptionsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Map inspector",
                    tooltip: "Inspect the raw map data (only available in debug mode)",
                    event: UserEvent::OpenMapDataWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Client state inspector",
                    tooltip: "Inspect and modify the internal client state (only available in debug mode)",
                    event: UserEvent::OpenClientStateInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Maps",
                    tooltip: "List of maps used for testing (only available in debug mode)",
                    event: UserEvent::OpenMapsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Commands",
                    tooltip: "List of commands used for testing (only available in debug mode)",
                    event: UserEvent::OpenCommandsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Time",
                    tooltip: "Time control (only available in debug mode)",
                    event: UserEvent::OpenTimeWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Theme inspector",
                    tooltip: "Inspect and edit the theme (only available in debug mode)",
                    event: UserEvent::OpenThemeInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Profiler",
                    tooltip: "Profile the client (only available in debug mode)",
                    event: UserEvent::OpenProfilerWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Packet inspector",
                    tooltip: "Inspect all incoming and outgoing packets (only available in debug mode)",
                    event: UserEvent::OpenPacketInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                button! {
                    text: "Log out",
                    event: UserEvent::LogOut,
                },
                button! {
                    text: "Exit",
                    event: UserEvent::Exit,
                },
            ),
        }
    }
}
