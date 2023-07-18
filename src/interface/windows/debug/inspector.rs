use procedural::*;

use crate::debug::Measurement;
use crate::interface::*;

pub struct FrameInspectorWindow {
    measurement: Measurement,
}

impl FrameInspectorWindow {
    pub fn new(measurement: Measurement) -> Self {
        Self { measurement }
    }
}

impl PrototypeWindow for FrameInspectorWindow {
    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let elements = vec![FrameInspectorView::new(self.measurement.clone()).wrap()];

        WindowBuilder::default()
            .with_title("Frame Inspector".to_string())
            .with_size(constraint!(200 > 500 < 900, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
