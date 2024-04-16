use derive_new::new;
use korangar_interface::elements::{ButtonBuilder, ElementWrap};
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use ragnarok_networking::CharacterServerInformation;

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::theme::InterfaceThemeKind;
use crate::interface::windows::WindowCache;

#[derive(new)]
pub struct SelectServerWindow {
    servers: Vec<CharacterServerInformation>,
}

impl SelectServerWindow {
    pub const WINDOW_CLASS: &'static str = "service_server";
}

impl PrototypeWindow<InterfaceSettings> for SelectServerWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = self
            .servers
            .iter()
            .map(|server| {
                ButtonBuilder::new()
                    .with_text(server.server_name.clone())
                    .with_event(UserEvent::SelectServer(server.clone()))
                    .build()
                    .wrap()
            })
            .collect();

        WindowBuilder::new()
            .with_title("Select Server".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .with_theme_kind(InterfaceThemeKind::Menu)
            .build(window_cache, application, available_space)
    }
}
