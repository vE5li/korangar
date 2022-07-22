use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::InterfaceSettings;
use crate::interface::{ WindowCache, Size };
use crate::interface::FramedWindow;

pub struct ProfilerWindow {
    window_class: String,
}

impl Default for ProfilerWindow {
   
    fn default() -> Self {
        Self { window_class: "profiler".to_string() }
    }
}

impl PrototypeWindow for ProfilerWindow {

    fn window_class(&self) -> Option<&str> {
        Some(&self.window_class)
    } 

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements = vec![];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "Profiler".to_string(), self.window_class.clone().into(), elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
