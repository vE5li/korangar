use korangar_interface::elements::{ElementWrap, PickList, PrototypeElement, StateButtonBuilder, Text};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};
use rust_state::Context;

use crate::graphics::{PresentModeInfo, ShadowDetail};
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::GameState;

pub struct GraphicsSettingsWindow {
    present_mode_info: PresentModeInfo,
}

impl GraphicsSettingsWindow {
    pub const WINDOW_CLASS: &'static str = "graphics_settings";

    pub fn new(present_mode_info: PresentModeInfo) -> Self {
        Self { present_mode_info }
    }
}

impl PrototypeWindow<GameState> for GraphicsSettingsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        let mut elements = vec![
            Text::default().with_text("Shadow detail").with_width(dimension_bound!(50%)).wrap(),
            PickList::default()
                .with_options(vec![
                    ("Low", ShadowDetail::Low),
                    ("Medium", ShadowDetail::Medium),
                    ("High", ShadowDetail::High),
                    ("Ultra", ShadowDetail::Ultra),
                ])
                .with_selected(GameState::shadow_detail())
                .with_event(|_: &Context<GameState>| Vec::new())
                .with_width(dimension_bound!(!))
                .wrap(),
            // FIX: Put back
            // application.to_element("Interface settings".to_string()),
        ];

        // TODO: Instead of not showing this option, disable the checkbox and add a
        // tooltip
        if self.present_mode_info.supports_immediate || self.present_mode_info.supports_mailbox {
            elements.insert(
                0,
                StateButtonBuilder::new()
                    .with_text("Framerate limit")
                    .with_remote(GameState::framerate_limit())
                    .with_toggle_event()
                    .build()
                    .wrap(),
            );
        }

        WindowBuilder::new()
            .with_title("Graphics Settings".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
