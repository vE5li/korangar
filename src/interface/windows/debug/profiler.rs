use procedural::*;

use crate::interface::*;

pub struct ProfilerWindow {
    always_update: TrackedState<bool>,
    visible_thread: TrackedState<ProfilerThread>,
}

impl ProfilerWindow {
    pub const WINDOW_CLASS: &'static str = "profiler";

    pub fn new() -> Self {
        Self {
            always_update: TrackedState::new(true),
            visible_thread: TrackedState::new(ProfilerThread::Main),
        }
    }
}

impl PrototypeWindow for ProfilerWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let toggle_halting = || {
            let is_profiler_halted = is_profiler_halted();
            set_profiler_halted(!is_profiler_halted);
            Vec::new()
        };

        let elements = vec![
            PickList::default()
                .with_options(vec![
                    ("Main thread", ProfilerThread::Main),
                    ("Picker thread", ProfilerThread::Picker),
                    ("Shadow thread", ProfilerThread::Shadow),
                    ("Deferred thread", ProfilerThread::Deferred),
                ])
                .with_selected(self.visible_thread.clone())
                .with_width(dimension!(150))
                .with_event(Box::new(Vec::new))
                .wrap(),
            StateButton::default()
                .with_text("Always update")
                .with_selector(self.always_update.selector())
                .with_event(self.always_update.toggle_action())
                .with_width(dimension!(150))
                .wrap(),
            StateButton::default()
                .with_text("Halt")
                .with_selector(|_: &StateProvider| is_profiler_halted())
                .with_event(Box::new(toggle_halting))
                .with_width(dimension!(150))
                .wrap(),
            ElementWrap::wrap(FrameView::new(
                self.always_update.new_remote(),
                self.visible_thread.new_remote(),
            )),
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
