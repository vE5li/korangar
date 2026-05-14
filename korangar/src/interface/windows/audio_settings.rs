use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{AudioSettings, AudioSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

#[derive(Default)]
pub struct AudioSettingsWindow<A> {
    audio_settings_path: A,
}

impl<A> AudioSettingsWindow<A> {
    pub fn new(audio_settings_path: A) -> Self {
        Self { audio_settings_path }
    }
}

impl<A> CustomWindow<ClientState> for AudioSettingsWindow<A>
where
    A: Path<ClientState, AudioSettings>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::AudioSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().audio_settings_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                split! {
                    children: (
                        text! {
                            text: "Output device",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        drop_down! {
                            selected: client_state().audio_settings().selected_output_device(),
                            options: client_state().audio_settings().available_output_devices(),
                        },
                    ),
                },
                state_button! {
                    text: client_state().localization().mute_audio_on_focus_loss_button_text(),
                    state: self.audio_settings_path.mute_on_focus_loss(),
                    event: Toggle(self.audio_settings_path.mute_on_focus_loss()),
                },
            ),
        }
    }
}
