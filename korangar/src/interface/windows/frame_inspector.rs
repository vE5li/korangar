use korangar_debug::profiling::FrameMeasurement;
use korangar_interface::window::{CustomWindow, WindowTrait};
use rust_state::Path;

use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

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
    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Frame Inspector",
            theme: InterfaceThemeType::Game,
            closable: true,
            elements: (),
        }
    }
}
