use korangar_interface::elements::{ElementWrap, StateButtonBuilder};
use korangar_interface::size_bound;
use korangar_interface::state::TrackedStateBinary;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};

use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

#[derive(Default)]
pub struct AudioSettingsWindow<MuteOnFocusLoss>
where
    MuteOnFocusLoss: TrackedStateBinary<bool>,
{
    mute_on_focus_loss: MuteOnFocusLoss,
}

impl<MuteOnFocusLoss> AudioSettingsWindow<MuteOnFocusLoss>
where
    MuteOnFocusLoss: TrackedStateBinary<bool>,
{
    pub const WINDOW_CLASS: &'static str = "audio_settings";

    pub fn new(mute_on_focus_loss: MuteOnFocusLoss) -> Self {
        Self { mute_on_focus_loss }
    }
}

impl<MuteOnFocusLoss> PrototypeWindow<InterfaceSettings> for AudioSettingsWindow<MuteOnFocusLoss>
where
    MuteOnFocusLoss: TrackedStateBinary<bool>,
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
        let elements = vec![StateButtonBuilder::new()
            .with_text("Mute audio on focus loss")
            .with_event(self.mute_on_focus_loss.toggle_action())
            .with_remote(self.mute_on_focus_loss.new_remote())
            .build()
            .wrap()];

        WindowBuilder::new()
            .with_title("Audio Settings".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
