use derive_new::new;
use procedural::*;

use crate::graphics::Color;
use crate::interface::{ElementCell, FramedWindow, InterfaceSettings, PrototypeWindow, Size, Window, WindowCache, *};

#[derive(new)]
pub struct ErrorWindow {
    message: String,
}

impl PrototypeWindow for ErrorWindow {
    fn to_window(
        &self,
        window_cache: &WindowCache,
        interface_settings: &InterfaceSettings,
        avalible_space: Size,
    ) -> Box<dyn Window + 'static> {
        let elements: Vec<ElementCell> = vec![cell!(Text::new(
            self.message.clone(),
            Color::rgb(220, 100, 100),
            14.0,
            constraint!(100%, 14)
        ))];

        Box::from(FramedWindow::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Error".to_string(),
            None,
            elements,
            constraint!(300 > 400 < 500, ?),
            true,
        ))
    }
}
