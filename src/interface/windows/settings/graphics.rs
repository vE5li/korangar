use procedural::*;

use crate::graphics::{PresentModeInfo, ShadowDetail};
use crate::input::UserEvent;
use crate::interface::*;

#[derive(new)]
pub struct GraphicsSettingsWindow {
    present_mode_info: PresentModeInfo,
    shadow_detail: TrackedState<ShadowDetail>,
}

impl GraphicsSettingsWindow {
    pub const WINDOW_CLASS: &'static str = "graphics_settings";
}

impl PrototypeWindow for GraphicsSettingsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let mut elements = vec![
            Text::default().with_text("Shadow detail").with_width(dimension!(50%)).wrap(),
            PickList::default()
                .with_options(vec![
                    ("Low", ShadowDetail::Low),
                    ("Medium", ShadowDetail::Medium),
                    ("High", ShadowDetail::High),
                    ("Ultra", ShadowDetail::Ultra),
                ])
                .with_selected(self.shadow_detail.clone())
                .with_event(Box::new(|| Vec::new()))
                .with_width(dimension!(!))
                .wrap(),
            interface_settings.to_element("Interface settings".to_string()),
        ];

        // TODO: Instead of not showing this option, disable the checkbox and add a
        // tooltip
        if self.present_mode_info.supports_immediate || self.present_mode_info.supports_mailbox {
            elements.insert(
                0,
                StateButton::default()
                    .with_text("Framerate limit")
                    .with_selector(|state_provider| state_provider.graphics_settings.frame_limit)
                    .with_event(UserEvent::ToggleFrameLimit)
                    .wrap(),
            );
        }

        WindowBuilder::default()
            .with_title("Graphics Settings".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(200 > 250 < 300, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
