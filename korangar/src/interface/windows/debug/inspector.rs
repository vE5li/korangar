use korangar_debug::Measurement;
use korangar_interface::elements::ElementWrap;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_procedural::size_bound;

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::FrameInspectorView;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

pub struct FrameInspectorWindow {
    measurement: Measurement,
}

impl FrameInspectorWindow {
    pub fn new(measurement: Measurement) -> Self {
        Self { measurement }
    }
}

impl PrototypeWindow<InterfaceSettings> for FrameInspectorWindow {
    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![FrameInspectorView::new(self.measurement.clone()).wrap()];

        WindowBuilder::new()
            .with_title("Frame Inspector".to_string())
            .with_size_bound(size_bound!(200 > 500 < 900, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
