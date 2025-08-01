use korangar_interface::window::{CustomWindow, WindowTrait};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::settings::{AudioSettings, AudioSettingsPathExt};
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

#[derive(Default)]
pub struct AudioSettingsWindow<P> {
    path: P,
}

impl<P> AudioSettingsWindow<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P> CustomWindow<ClientState> for AudioSettingsWindow<P>
where
    P: Path<ClientState, AudioSettings>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::AudioSettings)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Audio Settings",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            elements: (
                state_button! {
                    text: "Mute audio on focus loss",
                    state: self.path.mute_on_focus_loss(),
                    event: Toggle(self.path.mute_on_focus_loss()),
                },
            ),
        }
    }
}
