use derive_new::new;

use crate::input::UserEvent;
use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::InterfaceSettings;
use crate::interface::elements::*;
use crate::interface::{ WindowCache, FramedWindow, ElementCell, Size };

#[derive(new)]
pub struct MapsWindow<'a> {
    map_files: &'a Vec<String>,
    #[new(value = "\"maps\".to_string()")]
    window_class: String,
}

impl<'a> PrototypeWindow for MapsWindow<'a> {

    fn window_class(&self) -> Option<&str> {
        Some(&self.window_class)
    } 

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements = self.map_files
            .iter()
            .map(|name| cell!(EventButton::new(name.to_string(), UserEvent::LoadNewMap(name.to_string()))) as ElementCell)
            .collect();
        
        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "maps".to_string(), self.window_class.clone().into(), elements, constraint!(200.0 > 250.0 < 300.0, ? < 80.0%)))
    }
}
