use korangar_interface::event::Toggle;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::{Context, Path};

use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::settings::{AudioSettings, AudioSettingsPathExt};
use crate::state::{ClientState, ClientThemeType};

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

    fn to_window<'a>(
        self,
        state: &Context<ClientState>,
        window_cache: &WindowCache,
        available_space: ScreenSize,
    ) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Audio Settings",
            class: Some(WindowClass::AudioSettings),
            theme: ClientThemeType::Game,
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
