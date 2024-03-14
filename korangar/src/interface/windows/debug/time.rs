use korangar_interface::elements::{ButtonBuilder, ElementWrap};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_procedural::size_bound;

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

#[derive(Default)]
pub struct TimeWindow;

impl TimeWindow {
    pub const WINDOW_CLASS: &'static str = "time";
}

impl PrototypeWindow<InterfaceSettings> for TimeWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
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
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
