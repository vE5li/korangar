use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
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
                        text: "Lighting mode",
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.lighting_mode(),
                        options: self.capabilities_path.lighting_modes(),
                    }
                )
            },
            state_button! {
                text: "Triple buffering",
                state: self.settings_path.triple_buffering(),
                event: Toggle(self.settings_path.triple_buffering()),
            },
            state_button! {
                text: "Enable VSYNC",
                state: self.settings_path.vsync(),
                event: Toggle(self.settings_path.vsync()),
                disabled: self.capabilities_path.vsync_setting_disabled(),
                disabled_tooltip: "This setting is not supported on your system",
            },
            split! {
                children: (
                    text! {
                        text: "Limit framerate",
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.limit_framerate(),
                        options: self.capabilities_path.limit_framerate_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Texture filtering",
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.texture_filtering(),
                        options: self.capabilities_path.texture_filtering_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Multisampling",
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.msaa(),
                        options: self.capabilities_path.supported_msaa(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Supersampling",
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.ssaa(),
                        options: self.capabilities_path.ssaa_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Screen space AA",
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.screen_space_anti_aliasing(),
                        options: self.capabilities_path.screen_space_anti_aliasing_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Shadow quality",
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.shadow_quality(),
                        options: self.capabilities_path.shadow_quality_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: "Shadow detail",
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.shadow_detail(),
                        options: self.capabilities_path.shadow_detail_options(),
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
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements,
        }
    }
}
