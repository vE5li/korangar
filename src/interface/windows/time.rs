use procedural::*;

use crate::input::UserEvent;
use crate::interface::{ElementCell, FramedWindow, InterfaceSettings, PrototypeWindow, Size, Window, WindowCache, *};

#[derive(Default)]
pub struct TimeWindow {}

impl TimeWindow {

    pub const WINDOW_CLASS: &'static str = "time";
}

impl PrototypeWindow for TimeWindow {

    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
    ) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            cell!(Button::new("set dawn", UserEvent::SetDawn, false)),
            cell!(Button::new("set noon", UserEvent::SetNoon, false)),
            cell!(Button::new("set dusk", UserEvent::SetDusk, false)),
            cell!(Button::new("set midnight", UserEvent::SetMidnight, false)),
        ];

        Box::from(FramedWindow::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Time".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(200 > 250 < 300, ? < 80%),
        ))
    }
}
