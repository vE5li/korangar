use procedural::*;

use crate::input::UserEvent;
use crate::interface::traits::{ Window, PrototypeWindow, PrototypeElement };
use crate::interface::types::InterfaceSettings;
use crate::interface::{ StateProvider, WindowCache, FramedWindow, ElementCell, Size };
use crate::interface::elements::StateButton;

#[derive(Default)]
pub struct GraphicsSettingsWindow {}

impl GraphicsSettingsWindow {

    pub const WINDOW_CLASS: &'static str = "graphics_settings";
}

impl PrototypeWindow for GraphicsSettingsWindow {

    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            { // TODO: replace with macro
                let selector = Box::new(|state_provider: &StateProvider| state_provider.render_settings.frame_limit);
                cell!(StateButton::new("framerate limit", UserEvent::ToggleFrameLimit, selector))
            },
            interface_settings.to_element("interface settings".to_string()),
        ];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "Graphics Settings".to_string(), Self::WINDOW_CLASS.to_string().into(), elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
