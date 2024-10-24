#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LimitFramerate {
    Unlimited,
    Limit(u16),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextureSamplerType {
    Nearest,
    Linear,
    Anisotropic(u16),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShadowDetail {
    Low,
    Medium,
    High,
    Ultra,
}

impl ShadowDetail {
    pub fn directional_shadow_resolution(self) -> u32 {
        match self {
            ShadowDetail::Low => 512,
            ShadowDetail::Medium => 1024,
            ShadowDetail::High => 2048,
            ShadowDetail::Ultra => 8192,
        }
    }

    pub fn point_shadow_resolution(self) -> u32 {
        match self {
            ShadowDetail::Low => 64,
            ShadowDetail::Medium => 128,
            ShadowDetail::High => 256,
            ShadowDetail::Ultra => 512,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub vsync: bool,
    pub limit_framerate: LimitFramerate,
    pub triple_buffering: bool,
    pub texture_filtering: TextureSamplerType,
    pub shadow_detail: ShadowDetail,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            vsync: true,
            limit_framerate: LimitFramerate::Unlimited,
            triple_buffering: true,
            texture_filtering: TextureSamplerType::Linear,
            shadow_detail: ShadowDetail::High,
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
        std::fs::write(Self::FILE_NAME, data).expect("unable to write file");
    }
}

impl Drop for GraphicsSettings {
    fn drop(&mut self) {
        self.save();
    }
}
