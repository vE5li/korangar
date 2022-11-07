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

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        let elements: Vec<ElementCell> = vec![
            Button::default().with_static_text("set dawn").with_event(UserEvent::SetDawn).wrap(),
            Button::default().with_static_text("set noon").with_event(UserEvent::SetNoon).wrap(),
            Button::default().with_static_text("set dusk").with_event(UserEvent::SetDusk).wrap(),
            Button::default()
                .with_static_text("set midnight")
                .with_event(UserEvent::SetMidnight)
                .wrap(),
        ];

        Window::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Time".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(200 > 250 < 300, ? < 80%),
            true,
        )
    }
}
