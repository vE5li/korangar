use interface::traits::{ Window, PrototypeWindow };
use interface::types::InterfaceSettings;
use interface::{ WindowCache, Size };
use interface::FramedWindow;

pub struct AudioSettingsWindow {
    window_class: String,
}

impl Default for AudioSettingsWindow {
   
    fn default() -> Self {
        Self { window_class: "audio_settings".to_string() }
    }
}

impl PrototypeWindow for AudioSettingsWindow {

    fn window_class(&self) -> Option<&str> {
        Some(&self.window_class)
    } 

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements = vec![];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "audio settigs".to_string(), self.window_class.clone().into(), elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
