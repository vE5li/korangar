use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{InterfaceSettings, InterfaceSettingsCapabilities, InterfaceSettingsCapabilitiesPathExt, InterfaceSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct InterfaceSettingsWindow<A, B> {
    settings_path: A,
    capabilities_path: B,
}

impl<A, B> InterfaceSettingsWindow<A, B> {
    pub fn new(settings_path: A, capabilities_path: B) -> Self {
        Self {
            settings_path,
            capabilities_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for InterfaceSettingsWindow<A, B>
where
    A: Path<ClientState, InterfaceSettings>,
    B: Path<ClientState, InterfaceSettingsCapabilities>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::InterfaceSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (
            split! {
                children: (
                    text! {
                        text: client_state().localization().language_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.language(),
                        options: self.capabilities_path.languages(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().scaling_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.scaling(),
                        options: self.capabilities_path.scalings(),
                    }
                )
            },
        );

        window! {
            title: client_state().localization().interface_settings_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            elements,
        }
    }
}
