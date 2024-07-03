use korangar_interface::elements::{ButtonBuilder, ElementWrap};
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use rust_state::Context;

use crate::input::UserEvent;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::GameState;

#[derive(Default)]
pub struct TimeWindow;

impl TimeWindow {
    pub const WINDOW_CLASS: &'static str = "time";
}

impl PrototypeWindow<GameState> for TimeWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
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
