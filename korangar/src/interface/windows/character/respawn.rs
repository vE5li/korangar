use korangar_interface::elements::{ButtonBuilder, ElementWrap};
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

#[derive(Default)]
pub struct RespawnWindow;

impl RespawnWindow {
    pub const WINDOW_CLASS: &'static str = "respawn";
}

impl PrototypeWindow<InterfaceSettings> for RespawnWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![
            ButtonBuilder::new()
                .with_text("Respawn")
                .with_event(UserEvent::Respawn)
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Disconnect")
                .with_event(UserEvent::LogOut)
                .build()
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Respawn Menu".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
