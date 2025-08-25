#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::{
    LimitFramerate, Msaa, PresentModeInfo, ScreenSpaceAntiAliasing, ShadowDetail, ShadowQuality, Ssaa, TextureSamplerType,
};

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct GraphicsSettings {
    pub lighting_mode: LightingMode,
    pub vsync: bool,
    pub limit_framerate: LimitFramerate,
    pub triple_buffering: bool,
    pub texture_filtering: TextureSamplerType,
    pub msaa: Msaa,
    pub ssaa: Ssaa,
    pub screen_space_anti_aliasing: ScreenSpaceAntiAliasing,
    pub shadow_detail: ShadowDetail,
    pub shadow_quality: ShadowQuality,
    pub high_quality_interface: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            lighting_mode: LightingMode::Enhanced,
            vsync: true,
            limit_framerate: LimitFramerate::Unlimited,
            triple_buffering: true,
            texture_filtering: TextureSamplerType::Anisotropic(4),
            msaa: Msaa::X4,
            ssaa: Ssaa::Off,
            screen_space_anti_aliasing: ScreenSpaceAntiAliasing::Off,
            shadow_detail: ShadowDetail::Normal,
            shadow_quality: ShadowQuality::SoftPCSSx16,
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
                "failed to save graphics settings to {}: {}",
                Self::FILE_NAME.magenta(),
                _error.to_string().red()
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

#[derive(RustState, StateElement)]
pub struct GraphicsSettingsCapabilities {
    lighting_modes: Vec<LightingMode>,
    texture_filtering_options: Vec<TextureSamplerType>,
    limit_framerate_options: Vec<LimitFramerate>,
    supported_msaa: Vec<Msaa>,
    ssaa_options: Vec<Ssaa>,
    screen_space_anti_aliasing_options: Vec<ScreenSpaceAntiAliasing>,
    shadow_quality_options: Vec<ShadowQuality>,
    shadow_detail_options: Vec<ShadowDetail>,
    vsync_setting_disabled: bool,
}

impl Default for GraphicsSettingsCapabilities {
    fn default() -> Self {
        Self {
            lighting_modes: vec![LightingMode::Classic, LightingMode::Enhanced],
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
            shadow_quality_options: vec![
                ShadowQuality::Hard,
                ShadowQuality::SoftPCF,
                ShadowQuality::SoftPCSSx8,
                ShadowQuality::SoftPCSSx16,
                ShadowQuality::SoftPCSSx32,
                ShadowQuality::SoftPCSSx64,
            ],
            shadow_detail_options: vec![ShadowDetail::Normal, ShadowDetail::Ultra, ShadowDetail::Insane],
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
