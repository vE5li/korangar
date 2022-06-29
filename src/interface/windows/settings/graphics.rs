use input::UserEvent;
use interface::traits::{ Window, PrototypeWindow, PrototypeElement };
use interface::types::InterfaceSettings;
use interface::{ StateProvider, WindowCache, FramedWindow, ElementCell, Size };
use interface::elements::StateButton;

pub struct GraphicsSettingsWindow {
    window_class: String,
}

impl Default for GraphicsSettingsWindow {
   
    fn default() -> Self {
        Self { window_class: "graphics_settigs".to_string() }
    }
}

impl PrototypeWindow for GraphicsSettingsWindow {

    fn window_class(&self) -> Option<&str> {
        Some(&self.window_class)
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![ 
            { // TODO: replace with macro
                let selector = Box::new(|state_provider: &StateProvider| state_provider.render_settings.frame_limit);
                cell!(StateButton::new("framerate limit", UserEvent::ToggleFrameLimit, selector))
            },
            interface_settings.to_element("interface settings".to_string()),
        ];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "graphics settigs".to_string(), self.window_class.clone().into(), elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
