use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use rust_state::Context;

use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::GameState;

#[derive(Default)]
pub struct AudioSettingsWindow;

impl AudioSettingsWindow {
    pub const WINDOW_CLASS: &'static str = "audio_settings";
}

impl PrototypeWindow<GameState> for AudioSettingsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        let elements = vec![];

        WindowBuilder::new()
            .with_title("Audio Settings".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
