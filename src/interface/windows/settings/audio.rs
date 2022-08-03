use procedural::*;

use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::InterfaceSettings;
use crate::interface::{ WindowCache, Size };
use crate::interface::FramedWindow;

#[derive(Default)]
pub struct AudioSettingsWindow {}

impl AudioSettingsWindow {

    pub const WINDOW_CLASS: &'static str = "audio_settings";
}

impl PrototypeWindow for AudioSettingsWindow {

    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements = vec![];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "Audio Settings".to_string(), Self::WINDOW_CLASS.to_string().into(), elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
