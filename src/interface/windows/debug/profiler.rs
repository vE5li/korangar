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

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let toggle_halting = || {
            let is_profiler_halted = is_profiler_halted();
            set_profiler_halted(!is_profiler_halted);
            None
        };

        let main_selector = {
            let visible_thread = self.visible_thread.clone();
            move |_: &StateProvider| *visible_thread.borrow() == ProfilerThread::Main
        };
        let show_main_thread = {
            let mut visible_thread = self.visible_thread.clone();
            move || {
                visible_thread.set(ProfilerThread::Main);
                None
            }
        };

        let picker_selector = {
            let visible_thread = self.visible_thread.clone();
            move |_: &StateProvider| *visible_thread.borrow() == ProfilerThread::Picker
        };
        let show_picker_thread = {
            let mut visible_thread = self.visible_thread.clone();
            move || {
                visible_thread.set(ProfilerThread::Picker);
                None
            }
        };

        let shadow_selector = {
            let visible_thread = self.visible_thread.clone();
            move |_: &StateProvider| *visible_thread.borrow() == ProfilerThread::Shadow
        };
        let show_shadow_thread = {
            let mut visible_thread = self.visible_thread.clone();
            move || {
                visible_thread.set(ProfilerThread::Shadow);
                None
            }
        };

        let deferred_selector = {
            let visible_thread = self.visible_thread.clone();
            move |_: &StateProvider| *visible_thread.borrow() == ProfilerThread::Deferred
        };
        let show_deferred_thread = {
            let mut visible_thread = self.visible_thread.clone();
            move || {
                visible_thread.set(ProfilerThread::Deferred);
                None
            }
        };

        let elements = vec![
            StateButton::default()
                .with_text("halt")
                .with_selector(|_: &StateProvider| is_profiler_halted())
                .with_event(Box::new(toggle_halting))
                .with_width(dimension!(20%))
                .wrap(),
            StateButton::default()
                .with_text("always update")
                .with_selector(self.always_update.selector())
                .with_event(self.always_update.toggle_action())
                .with_width(dimension!(20%))
                .wrap(),
            StateButton::default()
                .with_text("main thread")
                .with_selector(main_selector)
                .with_event(Box::new(show_main_thread))
                .with_width(dimension!(20%))
                .wrap(),
            StateButton::default()
                .with_text("picker thread")
                .with_selector(picker_selector)
                .with_event(Box::new(show_picker_thread))
                .with_width(dimension!(20%))
                .wrap(),
            StateButton::default()
                .with_text("shadow thread")
                .with_selector(shadow_selector)
                .with_event(Box::new(show_shadow_thread))
                .with_width(dimension!(20%))
                .wrap(),
            StateButton::default()
                .with_text("deferred thread")
                .with_selector(deferred_selector)
                .with_event(Box::new(show_deferred_thread))
                .with_width(dimension!(!))
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
