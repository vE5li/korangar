use derive_new::new;

use crate::graphics::Color;
use crate::interface::traits::{ Window, PrototypeWindow };
use crate::interface::types::{ InterfaceSettings, ChangeEvent };
use crate::interface::{ WindowCache, ElementCell, Size };
use crate::interface::elements::*;
use crate::interface::FramedWindow;

#[derive(new)]
pub struct ColorWindow {
    name: String,
    color_pointer: *const Color,
    change_event: Option<ChangeEvent>,
}

impl PrototypeWindow for ColorWindow {

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let rgb_elements: Vec<ElementCell> = vec![
            cell!(Headline::new("red".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.color_pointer).red as *const u8 }, 0, 255, self.change_event)),
            cell!(Headline::new("green".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.color_pointer).green as *const u8 }, 0, 255, self.change_event)),
            cell!(Headline::new("blue".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.color_pointer).blue as *const u8 }, 0, 255, self.change_event)),
        ];

        let elements: Vec<ElementCell> = vec![
            cell!(Expandable::new("rgb".to_string(), rgb_elements, true)),
        ];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, self.name.to_string(), None, elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
