use derive_new::new;

use crate::graphics::Color;
use crate::interface::*;

#[derive(new)]
pub struct ColorWindow {
    name: String,
    reference: &'static Color,
    change_event: Option<ChangeEvent>,
}

impl PrototypeWindow for ColorWindow {
    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let rgb_elements = vec![
            Headline::new("red".to_string(), Headline::DEFAULT_SIZE).wrap(),
            Slider::new(&self.reference.red, 0.0, 1.0, self.change_event).wrap(),
            Headline::new("green".to_string(), Headline::DEFAULT_SIZE).wrap(),
            Slider::new(&self.reference.green, 0.0, 1.0, self.change_event).wrap(),
            Headline::new("blue".to_string(), Headline::DEFAULT_SIZE).wrap(),
            Slider::new(&self.reference.blue, 0.0, 1.0, self.change_event).wrap(),
            Headline::new("alpha".to_string(), Headline::DEFAULT_SIZE).wrap(),
            Slider::new(&self.reference.alpha, 0.0, 1.0, self.change_event).wrap(),
        ];

        let elements = vec![Expandable::new("rgb".to_string(), rgb_elements, true).wrap()];

        WindowBuilder::new()
            .with_title(self.name.to_string())
            .with_size_bound(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
