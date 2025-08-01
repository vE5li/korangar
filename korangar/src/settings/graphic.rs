#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::{LimitFramerate, Msaa, ScreenSpaceAntiAliasing, ShadowDetail, ShadowQuality, Ssaa, TextureSamplerType};
use crate::loaders::Scaling;

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct GraphicsSettings {
    pub interface_scaling: Scaling,
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
            interface_scaling: Scaling::new(1.0),
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
            print_debug!("failed to load graphics configuration from {}", Self::FILE_NAME.magenta());

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading graphics configuration from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving graphics configuration to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write(Self::FILE_NAME, data).expect("unable to write file");
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
    interface_scalings: Vec<Scaling>,
    lighting_modes: Vec<LightingMode>,
    texture_filtering_options: Vec<TextureSamplerType>,
    limit_framerate_options: Vec<LimitFramerate>,
    supported_msaa: Vec<Msaa>,
    ssaa_options: Vec<Ssaa>,
    screen_space_anti_aliasing_options: Vec<ScreenSpaceAntiAliasing>,
    shadow_quality_options: Vec<ShadowQuality>,
    shadow_detail_options: Vec<ShadowDetail>,
    // TODO: Rename most likely.
    additional_settings_disabled: bool,
}

impl GraphicsSettingsCapabilities {
    pub fn new(supported_msaa: Vec<Msaa>, additional_settings_disabled: bool) -> Self {
        Self {
            interface_scalings: vec![
                Scaling::new(0.5),
                Scaling::new(0.6),
                Scaling::new(0.7),
                Scaling::new(0.8),
                Scaling::new(0.9),
                Scaling::new(1.0),
                Scaling::new(1.1),
                Scaling::new(1.2),
                Scaling::new(1.3),
                Scaling::new(1.4),
                Scaling::new(1.5),
                Scaling::new(1.6),
                Scaling::new(1.7),
                Scaling::new(1.8),
                Scaling::new(1.9),
                Scaling::new(2.0),
            ],
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
            supported_msaa,
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
            additional_settings_disabled,
        }
    }
}
