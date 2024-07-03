use derive_new::new;
use korangar_interface::elements::{ElementWrap, Expandable, Headline, Slider};
use korangar_interface::event::ChangeEvent;
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use rust_state::Context;

use crate::graphics::Color;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::GameState;

#[derive(new)]
pub struct ColorWindow {
    name: String,
    reference: &'static Color,
    change_event: Option<ChangeEvent>,
}

impl PrototypeWindow<GameState> for ColorWindow {
    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
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
