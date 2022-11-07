use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;

#[derive(Default)]
pub struct GraphicsSettingsWindow {}

impl GraphicsSettingsWindow {
    pub const WINDOW_CLASS: &'static str = "graphics_settings";
}

impl PrototypeWindow for GraphicsSettingsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Window {
        let elements: Vec<ElementCell> = vec![
            StateButton::default()
                .with_static_text("framerate limit")
                .with_selector(|state_provider| state_provider.render_settings.frame_limit)
                .with_event(UserEvent::ToggleFrameLimit)
                .wrap(),
            interface_settings.to_element("interface settings".to_string()),
        ];

        Window::new(
            window_cache,
            interface_settings,
            avalible_space,
            "Graphics Settings".to_string(),
            Self::WINDOW_CLASS.to_string().into(),
            elements,
            constraint!(200 > 250 < 300, ?),
            true,
        )
    }
}
