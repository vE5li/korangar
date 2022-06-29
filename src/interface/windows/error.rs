use derive_new::new;

use interface::traits::{ Window, PrototypeWindow };
use interface::types::InterfaceSettings;
use interface::elements::*;
use interface::{ WindowCache, FramedWindow, ElementCell, Size };
use graphics::Color;

#[derive(new)]
pub struct ErrorWindow {
    message: String,
}

impl PrototypeWindow for ErrorWindow {

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            cell!(Text::new(self.message.clone(), Color::rgb(220, 100, 100), 14.0, constraint!(100.0%, 14.0))),
        ];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "error".to_string(), None, elements, constraint!(300.0 > 400.0 < 500.0, ?)))
    }
}

