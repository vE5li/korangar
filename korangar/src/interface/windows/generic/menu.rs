use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::Context;

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};

#[derive(Default)]
pub struct MenuWindow;

impl CustomWindow<ClientState> for MenuWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Menu)
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (
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
                // TODO: debug_foreground_color
                foreground_color: theme().button().foreground_color(),
            },
            #[cfg(feature = "debug")]
            button! {
                text: "Map viewer",
                event: UserEvent::OpenMapDataWindow,
                // TODO: debug_foreground_color
                foreground_color: theme().button().foreground_color(),
            },
            #[cfg(feature = "debug")]
            button! {
                text: "Maps",
                event: UserEvent::OpenMapsWindow,
                // TODO: debug_foreground_color
                foreground_color: theme().button().foreground_color(),
            },
            #[cfg(feature = "debug")]
            button! {
                text: "Commands",
                event: UserEvent::OpenCommandsWindow,
                // TODO: debug_foreground_color
                foreground_color: theme().button().foreground_color(),
            },
            #[cfg(feature = "debug")]
            button! {
                text: "Time",
                event: UserEvent::OpenTimeWindow,
                // TODO: debug_foreground_color
                foreground_color: theme().button().foreground_color(),
            },
            #[cfg(feature = "debug")]
            button! {
                text: "Theme viewer",
                event: UserEvent::OpenThemeViewerWindow,
                // TODO: debug_foreground_color
                foreground_color: theme().button().foreground_color(),
            },
            #[cfg(feature = "debug")]
            button! {
                text: "Profiler",
                event: UserEvent::OpenProfilerWindow,
                // TODO: debug_foreground_color
                foreground_color: theme().button().foreground_color(),
            },
            #[cfg(feature = "debug")]
            button! {
                text: "Packets",
                event: UserEvent::OpenPacketWindow,
                // TODO: debug_foreground_color
                foreground_color: theme().button().foreground_color(),
            },
            button! {
                text: "Log out",
                event: UserEvent::LogOut,
            },
            button! {
                text: "Exit",
                event: UserEvent::Exit,
            },
        );

        window! {
            title: "Menu",
            class: Some(WindowClass::Menu),
            theme: ClientThemeType::Game,
            elements: elements,
        }
    }
}
