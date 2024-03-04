use derive_new::new;

use crate::input::UserEvent;
use crate::interface::*;
use crate::network::CharacterServerInformation;

#[derive(new)]
pub struct SelectServerWindow {
    servers: Vec<CharacterServerInformation>,
}

impl SelectServerWindow {
    pub const WINDOW_CLASS: &'static str = "service_server";
}

impl PrototypeWindow for SelectServerWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let elements = self
            .servers
            .iter()
            .map(|server| {
                Button::default()
                    .with_text(server.server_name.clone())
                    .with_event(UserEvent::SelectServer(server.clone()))
                    .wrap()
            })
            .collect();

        WindowBuilder::default()
            .with_title("Select Server".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(SizeBound::DEFAULT_UNBOUNDED)
            .with_elements(elements)
            .with_theme_kind(ThemeKind::Menu)
            .build(window_cache, interface_settings, available_space)
    }
}
