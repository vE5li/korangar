#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use super::ShadowDetail;

#[derive(Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub frame_limit: bool,
    pub shadow_detail: ShadowDetail,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            frame_limit: true,
            shadow_detail: ShadowDetail::Medium,
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
