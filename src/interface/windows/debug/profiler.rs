use procedural::*;

use crate::interface::*;

pub struct ProfilerWindow {
    always_update: TrackedState<bool>,
}

impl ProfilerWindow {
    pub const WINDOW_CLASS: &'static str = "profiler";

    pub fn new() -> Self {
        Self {
            always_update: TrackedState::new(true),
        }
    }
}

impl PrototypeWindow for ProfilerWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let elements = vec![
            StateButton::default()
                .with_text("always update")
                .with_selector(self.always_update.selector())
                .with_event(self.always_update.toggle_action())
                .with_width(dimension!(30%))
                .wrap(),
            ElementWrap::wrap(FrameViewer::new(self.always_update.new_remote())),
        ];

        WindowBuilder::default()
            .with_title("Profiler".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(200 > 500 < 900, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
