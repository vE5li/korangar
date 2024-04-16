use korangar_interface::elements::{ButtonBuilder, ElementWrap};
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::theme::InterfaceTheme;
use crate::interface::windows::WindowCache;

#[derive(Default)]
pub struct MenuWindow;

impl MenuWindow {
    pub const WINDOW_CLASS: &'static str = "menu";
}

impl PrototypeWindow<InterfaceSettings> for MenuWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![
            ButtonBuilder::new()
                .with_text("Graphics settings")
                .with_event(UserEvent::OpenGraphicsSettingsWindow)
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Audio settings")
                .with_event(UserEvent::OpenAudioSettingsWindow)
                .build()
                .wrap(),
            #[cfg(feature = "debug")]
            ButtonBuilder::new()
                .with_text("Render settings")
                .with_event(UserEvent::OpenRenderSettingsWindow)
                .with_foreground_color(|theme: &InterfaceTheme| theme.button.debug_foreground_color.get())
                .build()
                .wrap(),
            #[cfg(feature = "debug")]
            ButtonBuilder::new()
                .with_text("Map viewer")
                .with_event(UserEvent::OpenMapDataWindow)
                .with_foreground_color(|theme: &InterfaceTheme| theme.button.debug_foreground_color.get())
                .build()
                .wrap(),
            #[cfg(feature = "debug")]
            ButtonBuilder::new()
                .with_text("Maps")
                .with_event(UserEvent::OpenMapsWindow)
                .with_foreground_color(|theme: &InterfaceTheme| theme.button.debug_foreground_color.get())
                .build()
                .wrap(),
            #[cfg(feature = "debug")]
            ButtonBuilder::new()
                .with_text("Commands")
                .with_event(UserEvent::OpenCommandsWindow)
                .with_foreground_color(|theme: &InterfaceTheme| theme.button.debug_foreground_color.get())
                .build()
                .wrap(),
            #[cfg(feature = "debug")]
            ButtonBuilder::new()
                .with_text("Time")
                .with_event(UserEvent::OpenTimeWindow)
                .with_foreground_color(|theme: &InterfaceTheme| theme.button.debug_foreground_color.get())
                .build()
                .wrap(),
            #[cfg(feature = "debug")]
            ButtonBuilder::new()
                .with_text("Theme viewer")
                .with_event(UserEvent::OpenThemeViewerWindow)
                .with_foreground_color(|theme: &InterfaceTheme| theme.button.debug_foreground_color.get())
                .build()
                .wrap(),
            #[cfg(feature = "debug")]
            ButtonBuilder::new()
                .with_text("Profiler")
                .with_event(UserEvent::OpenProfilerWindow)
                .with_foreground_color(|theme: &InterfaceTheme| theme.button.debug_foreground_color.get())
                .build()
                .wrap(),
            #[cfg(feature = "debug")]
            ButtonBuilder::new()
                .with_text("Packets")
                .with_event(UserEvent::OpenPacketWindow)
                .with_foreground_color(|theme: &InterfaceTheme| theme.button.debug_foreground_color.get())
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Log out")
                .with_event(UserEvent::LogOut)
                .build()
                .wrap(),
            ButtonBuilder::new().with_text("Exit").with_event(UserEvent::Exit).build().wrap(),
        ];

        WindowBuilder::new()
            .with_title("Menu".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
