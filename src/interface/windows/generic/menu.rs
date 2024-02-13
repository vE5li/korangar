use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;

#[derive(Default)]
pub struct MenuWindow {}

impl MenuWindow {
    pub const WINDOW_CLASS: &'static str = "menu";
}

impl PrototypeWindow for MenuWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let elements = vec![
            Button::default()
                .with_text("Graphics settings")
                .with_event(UserEvent::OpenGraphicsSettingsWindow)
                .wrap(),
            Button::default()
                .with_text("Audio settings")
                .with_event(UserEvent::OpenAudioSettingsWindow)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_text("Render settings")
                .with_event(UserEvent::OpenRenderSettingsWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_text("Map viewer")
                .with_event(UserEvent::OpenMapDataWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_text("Maps")
                .with_event(UserEvent::OpenMapsWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_text("Commands")
                .with_event(UserEvent::OpenCommandsWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_text("Time")
                .with_event(UserEvent::OpenTimeWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_text("Theme viewer")
                .with_event(UserEvent::OpenThemeViewerWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_text("Profiler")
                .with_event(UserEvent::OpenProfilerWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_text("Packets")
                .with_event(UserEvent::OpenPacketWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            Button::default().with_text("Log out").with_event(UserEvent::LogOut).wrap(),
            Button::default().with_text("Exit").with_event(UserEvent::Exit).wrap(),
        ];

        WindowBuilder::default()
            .with_title("Menu".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(200 > 250 < 300, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
