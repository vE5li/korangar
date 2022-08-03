use procedural::*;

use crate::input::UserEvent;
use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::InterfaceSettings;
use crate::interface::elements::*;
use crate::interface::{ WindowCache, FramedWindow, ElementCell, Size };

#[derive(Default)]
pub struct MenuWindow {}

impl MenuWindow {

    pub const WINDOW_CLASS: &'static str = "menu";
}

impl PrototypeWindow for MenuWindow {

    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            cell!(Button::new("graphics settings", UserEvent::OpenGraphicsSettingsWindow, true)),
            cell!(Button::new("audio settings", UserEvent::OpenAudioSettingsWindow, true)),
            #[cfg(feature = "debug")]
            cell!(DebugButton::new("render settings", UserEvent::OpenRenderSettingsWindow, true)),
            #[cfg(feature = "debug")]
            cell!(DebugButton::new("map viewer", UserEvent::OpenMapDataWindow, true)),
            #[cfg(feature = "debug")]
            cell!(DebugButton::new("maps", UserEvent::OpenMapsWindow, true)),
            #[cfg(feature = "debug")]
            cell!(DebugButton::new("time", UserEvent::OpenTimeWindow, true)),
            #[cfg(feature = "debug")]
            cell!(DebugButton::new("theme viewer", UserEvent::OpenThemeViewerWindow, true)),
            #[cfg(feature = "debug")]
            cell!(DebugButton::new("profiler", UserEvent::OpenProfilerWindow, true)),
            cell!(Button::new("log out", UserEvent::LogOut, true)),
            cell!(Button::new("exit korangar", UserEvent::Exit, true)),
        ];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "Menu".to_string(), Self::WINDOW_CLASS.to_string().into(), elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
