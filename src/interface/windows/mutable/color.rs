use derive_new::new;
use procedural::*;

use crate::graphics::Color;
use crate::interface::*;

#[derive(new)]
pub struct ColorWindow {
    name: String,
    color_pointer: *const Color,
    change_event: Option<ChangeEvent>,
}

impl PrototypeWindow for ColorWindow {
    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        let rgb_elements: Vec<ElementCell> = vec![
            cell!(Headline::new("red".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(
                unsafe { &(*self.color_pointer).red as *const u8 },
                0,
                255,
                self.change_event
            )),
            cell!(Headline::new("green".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(
                unsafe { &(*self.color_pointer).green as *const u8 },
                0,
                255,
                self.change_event
            )),
            cell!(Headline::new("blue".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(
                unsafe { &(*self.color_pointer).blue as *const u8 },
                0,
                255,
                self.change_event
            )),
            cell!(Headline::new("alpha".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(
                unsafe { &(*self.color_pointer).alpha as *const u8 },
                0,
                255,
                self.change_event
            )),
        ];

        let elements: Vec<ElementCell> = vec![cell!(Expandable::new("rgb".to_string(), rgb_elements, true))];

        WindowBuilder::default()
            .with_title(self.name.to_string())
            .with_size(constraint!(200 > 250 < 300, ?))
            .with_elements(elements)
            .closeable()
            .build(window_cache, interface_settings, avalible_space)
    }
}
