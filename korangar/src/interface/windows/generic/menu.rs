use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::Context;

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemePathExt, ClientThemeType, DebugButtonThemePathExt, client_theme};

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
            theme: ClientThemeType::Game,
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
                    text: "Render settings",
                    event: UserEvent::OpenRenderSettingsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Map viewer",
                    event: UserEvent::OpenMapDataWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Maps",
                    event: UserEvent::OpenMapsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Commands",
                    event: UserEvent::OpenCommandsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Time",
                    event: UserEvent::OpenTimeWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Theme viewer",
                    event: UserEvent::OpenThemeViewerWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Profiler",
                    event: UserEvent::OpenProfilerWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: "Packets",
                    event: UserEvent::OpenPacketWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
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
