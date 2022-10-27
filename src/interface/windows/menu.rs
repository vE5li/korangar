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

    fn to_window(
        &self,
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
    ) -> Box<dyn Window + 'static> {
        let elements = vec![
            Button::default()
                .with_static_text("graphics settings")
                .with_event(UserEvent::OpenGraphicsSettingsWindow)
                .wrap(),
            Button::default()
                .with_static_text("audio settings")
                .with_event(UserEvent::OpenAudioSettingsWindow)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_static_text("render settings")
                .with_event(UserEvent::OpenRenderSettingsWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_static_text("map viewer")
                .with_event(UserEvent::OpenMapDataWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_static_text("maps")
                .with_event(UserEvent::OpenMapsWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_static_text("time")
                .with_event(UserEvent::OpenTimeWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_static_text("theme viewer")
                .with_event(UserEvent::OpenThemeViewerWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug")]
            Button::default()
                .with_static_text("profiler")
                .with_event(UserEvent::OpenProfilerWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            #[cfg(feature = "debug_network")]
            Button::default()
                .with_static_text("packets")
                .with_event(UserEvent::OpenPacketWindow)
                .with_foreground_color(|theme| *theme.button.debug_foreground_color)
                .wrap(),
            Button::default().with_static_text("log out").with_event(UserEvent::LogOut).wrap(),
            Button::default().with_static_text("exit").with_event(UserEvent::Exit).wrap(),
        ];

        Box::from(FramedWindow::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Menu".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(200 > 250 < 300, ?),
            true,
        ))
    }
}
