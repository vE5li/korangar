#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::graphics::{LimitFramerate, Msaa, ScreenSpaceAntiAliasing, ShadowDetail, ShadowQuality, Ssaa, TextureSamplerType};

#[derive(Serialize, Deserialize)]
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
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightingMode {
    /// Mode that mimics the way the original client rendered the game.
    Classic,
    /// Mode that enabled all enhanced graphics features.
    Enhanced,
}
