use procedural::*;

use crate::interface::{InterfaceSettings, PrototypeWindow, Size, Window, WindowCache};

#[derive(Default)]
pub struct AudioSettingsWindow {}

impl AudioSettingsWindow {
    pub const WINDOW_CLASS: &'static str = "audio_settings";
}

impl PrototypeWindow for AudioSettingsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        let elements = vec![];

        Window::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Audio Settings".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(200 > 250 < 300, ?),
            true,
        )
    }
}
