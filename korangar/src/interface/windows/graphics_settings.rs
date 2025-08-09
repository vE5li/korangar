use korangar_interface::components::drop_down::DefaultClickHandler;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::settings::{GraphicsSettingsCapabilitiesPathExt, GraphicsSettingsPathExt};
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::{GraphicsSettings, GraphicsSettingsCapabilities};

pub struct GraphicsSettingsWindow<A, B> {
    settings_path: A,
    capabilities_path: B,
}

impl<A, B> GraphicsSettingsWindow<A, B> {
    pub fn new(settings_path: A, capabilities_path: B) -> Self {
        Self {
            settings_path,
            capabilities_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for GraphicsSettingsWindow<A, B>
where
    A: Path<ClientState, GraphicsSettings>,
    B: Path<ClientState, GraphicsSettingsCapabilities>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::GraphicsSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (
            split! {
                children: (
                    text! {
                        text: "Interface scale",
                    },
                    drop_down! {
                        selected: self.settings_path.interface_scaling(),
                        options: self.capabilities_path.interface_scalings(),
                        click_handler: DefaultClickHandler::new(self.settings_path.interface_scaling(), self.capabilities_path.interface_scalings()),
                    }
                )
            },
            state_button! {
                text: "Triple buffering",
                state: self.settings_path.triple_buffering(),
                event: Toggle(self.settings_path.triple_buffering()),
            },
            split! {
                children: (
                    text! {
                        text: "Lighting mode",
                    },
                    drop_down! {
                        selected: self.settings_path.lighting_mode(),
                        options: self.capabilities_path.lighting_modes(),
                        click_handler: DefaultClickHandler::new(self.settings_path.lighting_mode(), self.capabilities_path.lighting_modes()),
                    }
                )
            },
            state_button! {
                text: "Enable VSYNC",
                state: self.settings_path.vsync(),
                event: Toggle(self.settings_path.vsync()),
                disabled: self.capabilities_path.additional_settings_disabled(),
            },
            split! {
                children: (
                    text! {
                        text: "Limit framerate",
                    },
                    drop_down! {
                        selected: self.settings_path.limit_framerate(),
                        options: self.capabilities_path.limit_framerate_options(),
                        click_handler: DefaultClickHandler::new(self.settings_path.limit_framerate(), self.capabilities_path.limit_framerate_options()),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Texture filtering",
                    },
                    drop_down! {
                        selected: self.settings_path.texture_filtering(),
                        options: self.capabilities_path.texture_filtering_options(),
                        click_handler: DefaultClickHandler::new(self.settings_path.texture_filtering(), self.capabilities_path.texture_filtering_options()),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Multisampling",
                    },
                    drop_down! {
                        selected: self.settings_path.msaa(),
                        options: self.capabilities_path.supported_msaa(),
                        click_handler: DefaultClickHandler::new(self.settings_path.msaa(), self.capabilities_path.supported_msaa()),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Supersampling",
                    },
                    drop_down! {
                        selected: self.settings_path.ssaa(),
                        options: self.capabilities_path.ssaa_options(),
                        click_handler: DefaultClickHandler::new(self.settings_path.ssaa(), self.capabilities_path.ssaa_options()),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Screen space AA",
                    },
                    drop_down! {
                        selected: self.settings_path.screen_space_anti_aliasing(),
                        options: self.capabilities_path.screen_space_anti_aliasing_options(),
                        click_handler: DefaultClickHandler::new(self.settings_path.screen_space_anti_aliasing(), self.capabilities_path.screen_space_anti_aliasing_options()),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Shadow quality",
                    },
                    drop_down! {
                        selected: self.settings_path.shadow_quality(),
                        options: self.capabilities_path.shadow_quality_options(),
                        click_handler: DefaultClickHandler::new(self.settings_path.shadow_quality(), self.capabilities_path.shadow_quality_options()),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Shadow detail",
                    },
                    drop_down! {
                        selected: self.settings_path.shadow_detail(),
                        options: self.capabilities_path.shadow_detail_options(),
                        click_handler: DefaultClickHandler::new(self.settings_path.shadow_detail(), self.capabilities_path.shadow_detail_options()),
                    }
                )
            },
            state_button! {
                text: "High quality interface",
                state: self.settings_path.high_quality_interface(),
                event: Toggle(self.settings_path.high_quality_interface()),
            },
        );

        window! {
            title: "Graphics Settings",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            elements: elements,
        }
    }
}
