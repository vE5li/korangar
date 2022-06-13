use derive_new::new;

use graphics::Color;
use interface::traits::{ Window, PrototypeWindow };
use interface::types::InterfaceSettings;
use interface::{ WindowCache, ElementCell, Size };
use interface::elements::*;
use interface::FramedWindow;

#[derive(new)]
pub struct ColorWindow {
    name: String,
    color_pointer: *const Color,
}

impl PrototypeWindow for ColorWindow {

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let rgb_elements: Vec<ElementCell> = vec![
            cell!(Headline::new("red".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.color_pointer).red as *const u8 }, 0, 255)),
            cell!(Headline::new("green".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.color_pointer).green as *const u8 }, 0, 255)),
            cell!(Headline::new("blue".to_string(), Headline::DEFAULT_SIZE)),
            cell!(Slider::new(unsafe { &(*self.color_pointer).blue as *const u8 }, 0, 255)),
        ];

        let elements: Vec<ElementCell> = vec![
            cell!(Headline::new(self.name.clone(), Headline::DEFAULT_SIZE)),
            cell!(Expandable::new("rgb".to_string(), rgb_elements, true)),
        ];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "color picker".to_string(), None, elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
