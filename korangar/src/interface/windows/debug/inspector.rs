use korangar_debug::profiling::Measurement;
use korangar_interface::elements::ElementWrap;
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use rust_state::Context;

use crate::interface::elements::FrameInspectorView;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::GameState;

pub struct FrameInspectorWindow {
    measurement: Measurement,
}

impl FrameInspectorWindow {
    pub fn new(measurement: Measurement) -> Self {
        Self { measurement }
    }
}

impl PrototypeWindow<GameState> for FrameInspectorWindow {
    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        let elements = vec![FrameInspectorView::new(self.measurement.clone()).wrap()];

        WindowBuilder::new()
            .with_title("Frame Inspector".to_string())
            .with_size_bound(size_bound!(200 > 500 < 900, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
