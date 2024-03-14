use korangar_interface::elements::{ElementWrap, PickList, PrototypeElement, StateButtonBuilder, Text};
use korangar_interface::state::{TrackedState, TrackedStateBinary};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_procedural::{dimension_bound, size_bound};

use crate::graphics::{PresentModeInfo, ShadowDetail};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

pub struct GraphicsSettingsWindow<Shadow, Framerate>
where
    Shadow: TrackedState<ShadowDetail> + 'static,
    Framerate: TrackedStateBinary<bool>,
{
    present_mode_info: PresentModeInfo,
    shadow_detail: Shadow,
    framerate_limit: Framerate,
}

impl<Shadow, Framerate> GraphicsSettingsWindow<Shadow, Framerate>
where
    Shadow: TrackedState<ShadowDetail> + 'static,
    Framerate: TrackedStateBinary<bool>,
{
    pub const WINDOW_CLASS: &'static str = "graphics_settings";

    pub fn new(present_mode_info: PresentModeInfo, shadow_detail: Shadow, framerate_limit: Framerate) -> Self {
        Self {
            present_mode_info,
            shadow_detail,
            framerate_limit,
        }
    }
}

impl<Shadow, Framerate> PrototypeWindow<InterfaceSettings> for GraphicsSettingsWindow<Shadow, Framerate>
where
    Shadow: TrackedState<ShadowDetail> + 'static,
    Framerate: TrackedStateBinary<bool>,
{
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let mut elements = vec![
            Text::default().with_text("Shadow detail").with_width(dimension_bound!(50%)).wrap(),
            PickList::default()
                .with_options(vec![
                    ("Low", ShadowDetail::Low),
                    ("Medium", ShadowDetail::Medium),
                    ("High", ShadowDetail::High),
                    ("Ultra", ShadowDetail::Ultra),
                ])
                .with_selected(self.shadow_detail.clone())
                .with_event(Box::new(Vec::new))
                .with_width(dimension_bound!(!))
                .wrap(),
            application.to_element("Interface settings".to_string()),
        ];

        // TODO: Instead of not showing this option, disable the checkbox and add a
        // tooltip
        if self.present_mode_info.supports_immediate || self.present_mode_info.supports_mailbox {
            elements.insert(
                0,
                StateButtonBuilder::new()
                    .with_text("Framerate limit")
                    .with_event(self.framerate_limit.toggle_action())
                    .with_remote(self.framerate_limit.new_remote())
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
