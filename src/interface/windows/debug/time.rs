use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;

#[derive(Default)]
pub struct TimeWindow {}

impl TimeWindow {
    pub const WINDOW_CLASS: &'static str = "time";
}

impl PrototypeWindow for TimeWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let elements = vec![
            Button::default().with_text("set dawn").with_event(UserEvent::SetDawn).wrap(),
            Button::default().with_text("set noon").with_event(UserEvent::SetNoon).wrap(),
            Button::default().with_text("set dusk").with_event(UserEvent::SetDusk).wrap(),
            Button::default()
                .with_text("set midnight")
                .with_event(UserEvent::SetMidnight)
                .wrap(),
        ];

        WindowBuilder::default()
            .with_title("Time".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(200 > 250 < 300, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
