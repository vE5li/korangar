use derive_new::new;

use crate::input::UserEvent;
use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::InterfaceSettings;
use crate::interface::elements::*;
use crate::interface::{ WindowCache, FramedWindow, ElementCell, Size };
use crate::types::maths::Vector2;

#[derive(new)]
pub struct TimeWindow {
    #[new(value = "\"time\".to_string()")]
    window_class: String,
}

impl PrototypeWindow for TimeWindow {

    fn window_class(&self) -> Option<&str> {
        Some(&self.window_class)
    } 

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            cell!(Button::new("set dawn", UserEvent::SetDawn, false)),
            cell!(Button::new("set noon", UserEvent::SetNoon, false)),
            cell!(Button::new("set dusk", UserEvent::SetDusk, false)),
            cell!(Button::new("set midnight", UserEvent::SetMidnight, false)),
        ];
        
        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "Time".to_string(), self.window_class.clone().into(), elements, constraint!(200.0 > 250.0 < 300.0, ? < 80.0%)))
    }
}
