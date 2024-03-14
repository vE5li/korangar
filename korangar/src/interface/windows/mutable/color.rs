use derive_new::new;
use korangar_interface::elements::{ElementWrap, Expandable, Headline, Slider};
use korangar_interface::event::ChangeEvent;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_procedural::size_bound;

use crate::graphics::Color;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

#[derive(new)]
pub struct ColorWindow {
    name: String,
    reference: &'static Color,
    change_event: Option<ChangeEvent>,
}

impl PrototypeWindow<InterfaceSettings> for ColorWindow {
    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let rgb_elements = vec![
            Headline::new("red".to_string(), size_bound!(100%, 12)).wrap(),
            Slider::new(&self.reference.red, 0.0, 1.0, self.change_event).wrap(),
            Headline::new("green".to_string(), size_bound!(100%, 12)).wrap(),
            Slider::new(&self.reference.green, 0.0, 1.0, self.change_event).wrap(),
            Headline::new("blue".to_string(), size_bound!(100%, 12)).wrap(),
            Slider::new(&self.reference.blue, 0.0, 1.0, self.change_event).wrap(),
            Headline::new("alpha".to_string(), size_bound!(100%, 12)).wrap(),
            Slider::new(&self.reference.alpha, 0.0, 1.0, self.change_event).wrap(),
        ];

        let elements = vec![Expandable::new("rgb".to_string(), rgb_elements, true).wrap()];

        WindowBuilder::new()
            .with_title(self.name.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
