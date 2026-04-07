#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::{
    LimitFramerate, Msaa, PresentModeInfo, ScreenSpaceAntiAliasing, ShadowDetail, ShadowMethod, ShadowResolution, Ssaa, TextureSamplerType,
};

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct GraphicsSettings {
    pub lighting_mode: LightingMode,
    #[serde(default)]
    pub brightness: Brightness,
    pub vsync: bool,
    pub limit_framerate: LimitFramerate,
    pub triple_buffering: bool,
    pub texture_filtering: TextureSamplerType,
    pub msaa: Msaa,
    pub ssaa: Ssaa,
    pub screen_space_anti_aliasing: ScreenSpaceAntiAliasing,
    pub shadow_method: ShadowMethod,
    pub shadow_resolution: ShadowResolution,
    pub shadow_detail: ShadowDetail,
    pub sdsm: bool,
    pub high_quality_interface: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            lighting_mode: LightingMode::Enhanced,
            brightness: Brightness::B100,
            vsync: true,
            limit_framerate: LimitFramerate::Unlimited,
            triple_buffering: true,
            texture_filtering: TextureSamplerType::Anisotropic(4),
            msaa: Msaa::X4,
            ssaa: Ssaa::Off,
            screen_space_anti_aliasing: ScreenSpaceAntiAliasing::Off,
            shadow_method: ShadowMethod::SoftPCSS,
            shadow_resolution: ShadowResolution::Normal,
            shadow_detail: ShadowDetail::Medium,
            sdsm: true,
            high_quality_interface: true,
        }
    }
}

impl GraphicsSettings {
    const FILE_NAME: &'static str = "client/graphics_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load graphics settings from {}", Self::FILE_NAME.magenta());

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading graphics settings from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving graphics settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save graphics settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }
}

impl Drop for GraphicsSettings {
    fn drop(&mut self) {
        self.save();
    }
}

/// The lighting mode used when rendering the game.
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, StateElement)]
pub enum LightingMode {
    /// Mode that mimics the way the original client rendered the game.
    Classic,
    /// Mode that enabled all enhanced graphics features.
    Enhanced,
}

impl DropDownItem<LightingMode> for LightingMode {
    fn text(&self) -> &str {
        match self {
            LightingMode::Classic => "Classic",
            LightingMode::Enhanced => "Enhanced",
        }
    }

    fn value(&self) -> LightingMode {
        *self
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, StateElement)]
pub enum Brightness {
    B50,
    B60,
    B70,
    B80,
    B90,
    B100,
    B110,
    B120,
    B130,
    B140,
    B150,
    B160,
    B170,
    B180,
    B190,
    B200,
}

impl Default for Brightness {
    fn default() -> Self {
        Brightness::B100
    }
}

impl Brightness {
    pub fn factor(self) -> f32 {
        match self {
            Brightness::B50 => 0.5,
            Brightness::B60 => 0.6,
            Brightness::B70 => 0.7,
            Brightness::B80 => 0.8,
            Brightness::B90 => 0.9,
            Brightness::B100 => 1.0,
            Brightness::B110 => 1.1,
            Brightness::B120 => 1.2,
            Brightness::B130 => 1.3,
            Brightness::B140 => 1.4,
            Brightness::B150 => 1.5,
            Brightness::B160 => 1.6,
            Brightness::B170 => 1.7,
            Brightness::B180 => 1.8,
            Brightness::B190 => 1.9,
            Brightness::B200 => 2.0,
        }
    }
}

impl DropDownItem<Brightness> for Brightness {
    fn text(&self) -> &str {
        match self {
            Brightness::B50 => "50%",
            Brightness::B60 => "60%",
            Brightness::B70 => "70%",
            Brightness::B80 => "80%",
            Brightness::B90 => "90%",
            Brightness::B100 => "100%",
            Brightness::B110 => "110%",
            Brightness::B120 => "120%",
            Brightness::B130 => "130%",
            Brightness::B140 => "140%",
            Brightness::B150 => "150%",
            Brightness::B160 => "160%",
            Brightness::B170 => "170%",
            Brightness::B180 => "180%",
            Brightness::B190 => "190%",
            Brightness::B200 => "200%",
        }
    }

    fn value(&self) -> Brightness {
        *self
    }
}

#[derive(RustState, StateElement)]
pub struct GraphicsSettingsCapabilities {
    lighting_modes: Vec<LightingMode>,
    brightness_options: Vec<Brightness>,
    texture_filtering_options: Vec<TextureSamplerType>,
    limit_framerate_options: Vec<LimitFramerate>,
    supported_msaa: Vec<Msaa>,
    ssaa_options: Vec<Ssaa>,
    screen_space_anti_aliasing_options: Vec<ScreenSpaceAntiAliasing>,
    shadow_method_options: Vec<ShadowMethod>,
    shadow_resolution_options: Vec<ShadowResolution>,
    shadow_detail_options: Vec<ShadowDetail>,
    vsync_setting_disabled: bool,
}

impl Default for GraphicsSettingsCapabilities {
    fn default() -> Self {
        Self {
            lighting_modes: vec![LightingMode::Classic, LightingMode::Enhanced],
            brightness_options: vec![
                Brightness::B50,
                Brightness::B60,
                Brightness::B70,
                Brightness::B80,
                Brightness::B90,
                Brightness::B100,
                Brightness::B110,
                Brightness::B120,
                Brightness::B130,
                Brightness::B140,
                Brightness::B150,
                Brightness::B160,
                Brightness::B170,
                Brightness::B180,
                Brightness::B190,
                Brightness::B200,
            ],
            texture_filtering_options: vec![
                TextureSamplerType::Nearest,
                TextureSamplerType::Linear,
                TextureSamplerType::Anisotropic(4),
                TextureSamplerType::Anisotropic(8),
                TextureSamplerType::Anisotropic(16),
            ],
            limit_framerate_options: vec![
                LimitFramerate::Unlimited,
                LimitFramerate::Limit(30),
                LimitFramerate::Limit(60),
                LimitFramerate::Limit(120),
                LimitFramerate::Limit(144),
                LimitFramerate::Limit(240),
            ],
            supported_msaa: Vec::new(),
            ssaa_options: vec![Ssaa::Off, Ssaa::X2, Ssaa::X3, Ssaa::X4],
            screen_space_anti_aliasing_options: vec![ScreenSpaceAntiAliasing::Off, ScreenSpaceAntiAliasing::Fxaa],
            shadow_method_options: vec![ShadowMethod::Hard, ShadowMethod::SoftPCF, ShadowMethod::SoftPCSS],
            shadow_resolution_options: vec![ShadowResolution::Normal, ShadowResolution::Ultra, ShadowResolution::Insane],
            shadow_detail_options: vec![ShadowDetail::Low, ShadowDetail::Medium, ShadowDetail::High, ShadowDetail::Ultra],
            vsync_setting_disabled: true,
        }
    }
}

impl GraphicsSettingsCapabilities {
    pub fn update(&mut self, supported_msaa: Vec<Msaa>, present_mode_info: PresentModeInfo) {
        self.supported_msaa = supported_msaa;
        self.vsync_setting_disabled = !present_mode_info.supports_mailbox && !present_mode_info.supports_immediate;
    }
}
