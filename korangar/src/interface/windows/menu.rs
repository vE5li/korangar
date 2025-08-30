use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
#[cfg(feature = "debug")]
use crate::state::client_theme;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
#[cfg(feature = "debug")]
use crate::state::theme::{DebugButtonThemePathExt, InterfaceThemePathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state};

#[derive(Default)]
pub struct MenuWindow;

impl CustomWindow<ClientState> for MenuWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Menu)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().menu_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                button! {
                    text: client_state().localization().interface_settings_button_text(),
                    event: InputEvent::ToggleInterfaceSettingsWindow,
                },
                button! {
                    text: client_state().localization().graphics_settings_button_text(),
                    event: InputEvent::ToggleGraphicsSettingsWindow,
                },
                button! {
                    text: client_state().localization().audio_settings_button_text(),
                    event: InputEvent::ToggleAudioSettingsWindow,
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Render options",
                    tooltip: "Special render options (^000001only available in debug mode^000000)",
                    event: InputEvent::ToggleRenderOptionsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Map inspector",
                    tooltip: "Inspect the raw map data (^000001only available in debug mode^000000)",
                    event: InputEvent::OpenMapDataWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Client state inspector",
                    tooltip: "Inspect and modify the internal client state (^000001only available in debug mode^000000)",
                    event: InputEvent::ToggleClientStateInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Maps",
                    tooltip: "List of maps used for testing (^000001only available in debug mode^000000)",
                    event: InputEvent::ToggleMapsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Commands",
                    tooltip: "List of commands used for testing (^000001only available in debug mode^000000)",
                    event: InputEvent::ToggleCommandsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Time",
                    tooltip: "Time control (^000001only available in debug mode^000000)",
                    event: InputEvent::ToggleTimeWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Theme inspector",
                    tooltip: "Inspect and edit the theme (^000001only available in debug mode^000000)",
                    event: InputEvent::ToggleThemeInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Profiler",
                    tooltip: "Profile the client (^000001only available in debug mode^000000)",
                    event: InputEvent::ToggleProfilerWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Packet inspector",
                    tooltip: "Inspect all incoming and outgoing packets (^000001only available in debug mode^000000)",
                    event: InputEvent::TogglePacketInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Cache statistics",
                    tooltip: "Shows statistics of the caches used by the client (^000001only available in debug mode^000000)",
                    event: InputEvent::ToggleCacheStatisticsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                button! {
                    text: client_state().localization().log_out_button_text(),
                    event: InputEvent::LogOut,
                },
                button! {
                    text: client_state().localization().exit_button_text(),
                    event: InputEvent::Exit,
                },
            ),
        }
    }
}
