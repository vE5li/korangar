use korangar_debug::profiling::Profiler;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::{Context, Path};

use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};

pub struct ProfilerWindow<V> {
    visible_thread_path: V,
}

impl<V> ProfilerWindow<V> {
    pub fn new(visible_thread_path: V) -> Self {
        Self { visible_thread_path }
    }
}

impl<V> CustomWindow<ClientState> for ProfilerWindow<V>
where
    V: Path<ClientState, crate::threads::Enum>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Profiler)
    }

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Profiler",
            class: Self::window_class(),
            theme: ClientThemeType::Game,
            closable: true,
            elements: (),
        }
    }
}
