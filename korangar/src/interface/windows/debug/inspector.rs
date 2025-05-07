use korangar_debug::profiling::FrameMeasurement;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::{Context, Path};

use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::state::{ClientState, ClientThemeType};

pub struct FrameInspectorWindow<P> {
    path: P,
}

impl<P> FrameInspectorWindow<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P> CustomWindow<ClientState> for FrameInspectorWindow<P>
where
    P: Path<ClientState, FrameMeasurement>,
{
    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Frame Inspector",
            theme: ClientThemeType::Game,
            closable: true,
            elements: (),
        }
    }
}
