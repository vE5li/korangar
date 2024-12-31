use korangar_debug::profiling::Profiler;
use korangar_interface::elements::{ElementWrap, PickList, StateButtonBuilder};
use korangar_interface::state::{PlainTrackedState, Remote, TrackedState, TrackedStateBinary, ValueState};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::FrameView;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

/// Wrapper struct that exposes an implementation of [`TrackedState`] for the
/// halted state of the profiler.
#[derive(Default, Clone)]
struct TrackedProfilerHaltedState {
    dummy_state: std::rc::Rc<std::cell::RefCell<bool>>,
}

impl TrackedState<bool> for TrackedProfilerHaltedState {
    type RemoteType = ProfilerHaltedRemote;

    fn set(&mut self, value: bool) {
        Profiler::set_halted(value);
    }

    fn get(&self) -> std::cell::Ref<'_, bool> {
        *self.dummy_state.borrow_mut() = Profiler::get_halted();
        self.dummy_state.borrow()
    }

    fn with_mut<Closure, Return>(&mut self, closure: Closure) -> Return
    where
        Closure: FnOnce(&mut bool) -> korangar_interface::state::ValueState<Return>,
    {
        let mut temporary_state = *self.dummy_state.borrow();

        match closure(&mut temporary_state) {
            ValueState::Mutated(return_value) => {
                Profiler::set_halted(temporary_state);
                return_value
            }
            ValueState::Unchanged(return_value) => return_value,
        }
    }

    fn update(&mut self) {
        let state = Profiler::get_halted();
        Profiler::set_halted(state);
    }

    fn new_remote(&self) -> Self::RemoteType {
        let current_state = Profiler::get_halted();
        ProfilerHaltedRemote {
            state: self.clone(),
            current_state,
        }
    }
}

/// Wrapper struct that exposes an implementation of [`Remote`] for the halted
/// state of the profiler.
struct ProfilerHaltedRemote {
    state: TrackedProfilerHaltedState,
    current_state: bool,
}

impl Remote<bool> for ProfilerHaltedRemote {
    type State = TrackedProfilerHaltedState;

    fn clone_state(&self) -> Self::State {
        self.state.clone()
    }

    fn get(&self) -> std::cell::Ref<'_, bool> {
        self.state.get()
    }

    fn consume_changed(&mut self) -> bool {
        let new_state = Profiler::get_halted();
        let changed = self.current_state != new_state;
        self.current_state = new_state;

        changed
    }
}

pub struct ProfilerWindow {
    always_update: PlainTrackedState<bool>,
    visible_thread: PlainTrackedState<crate::threads::Enum>,
}

impl ProfilerWindow {
    pub const WINDOW_CLASS: &'static str = "profiler";

    pub fn new() -> Self {
        Self {
            always_update: PlainTrackedState::new(true),
            visible_thread: PlainTrackedState::new(crate::threads::Enum::Main),
        }
    }
}

impl PrototypeWindow<InterfaceSettings> for ProfilerWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let profiler_halted_state = TrackedProfilerHaltedState::default();

        let elements = vec![
            PickList::default()
                .with_options(vec![
                    ("Main thread", crate::threads::Enum::Main),
                    ("Loader thread", crate::threads::Enum::Loader),
                ])
                .with_selected(self.visible_thread.clone())
                .with_width(dimension_bound!(150))
                .with_event(Box::new(Vec::new))
                .wrap(),
            StateButtonBuilder::new()
                .with_text("Always update")
                .with_event(self.always_update.toggle_action())
                .with_remote(self.always_update.new_remote())
                .with_width_bound(dimension_bound!(150))
                .build()
                .wrap(),
            StateButtonBuilder::new()
                .with_text("Halt")
                .with_remote(profiler_halted_state.new_remote())
                .with_event(profiler_halted_state.toggle_action())
                .with_width_bound(dimension_bound!(150))
                .build()
                .wrap(),
            ElementWrap::wrap(FrameView::new(
                self.always_update.new_remote(),
                self.visible_thread.new_remote(),
            )),
        ];

        WindowBuilder::new()
            .with_title("Profiler".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 500 < 900, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
