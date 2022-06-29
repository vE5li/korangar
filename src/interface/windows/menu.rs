use input::UserEvent;
use interface::traits::{ Window, PrototypeWindow };
use interface::types::InterfaceSettings;
use interface::elements::*;
use interface::{ WindowCache, FramedWindow, ElementCell, Size };

pub struct MenuWindow {
    window_class: String,
}

impl Default for MenuWindow {
   
    fn default() -> Self {
        Self { window_class: "menu".to_string() }
    }
}

impl PrototypeWindow for MenuWindow {

    fn window_class(&self) -> Option<&str> {
        Some(&self.window_class)
    } 

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            cell!(Button::new("graphics settings", UserEvent::OpenGraphicsSettingsWindow, true)),
            cell!(Button::new("audio settings", UserEvent::OpenAudioSettingsWindow, true)),
            cell!(DebugButton::new("render settings", UserEvent::OpenRenderSettingsWindow, true)),
            cell!(DebugButton::new("map viewer", UserEvent::OpenMapDataWindow, true)),
            cell!(DebugButton::new("maps", UserEvent::OpenMapsWindow, true)),
            cell!(DebugButton::new("theme viewer", UserEvent::OpenThemeViewerWindow, true)),
            cell!(DebugButton::new("profiler", UserEvent::OpenProfilerWindow, true)),
            cell!(Button::new("exit korangar", UserEvent::Exit, true)),
        ];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "menu".to_string(), self.window_class.clone().into(), elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
