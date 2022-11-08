use procedural::*;

use crate::interface::{InterfaceSettings, PrototypeWindow, Size, Window, WindowBuilder, WindowCache};

#[derive(Default)]
pub struct ProfilerWindow {}

impl ProfilerWindow {
    pub const WINDOW_CLASS: &'static str = "profiler";
}

impl PrototypeWindow for ProfilerWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        WindowBuilder::default()
            .with_title("Profiler".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(200 > 250 < 300, ?))
            .closable()
            .build(window_cache, interface_settings, avalible_space)
    }
}
