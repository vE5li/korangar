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

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        // TODO: Unify Set* events into one that takes a specific time
        let elements = vec![
            ButtonBuilder::new()
                .with_text("Set dawn")
                .with_event(UserEvent::SetDawn)
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Set noon")
                .with_event(UserEvent::SetNoon)
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Set dusk")
                .with_event(UserEvent::SetDusk)
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Set midnight")
                .with_event(UserEvent::SetMidnight)
                .build()
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Time".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
