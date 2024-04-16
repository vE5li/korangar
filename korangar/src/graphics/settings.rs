use korangar_procedural::toggle;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use super::ShadowDetail;

#[derive(Serialize, Deserialize, toggle)]
pub struct GraphicsSettings {
    #[toggle]
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
    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            korangar_debug::print_debug!(
                "failed to load graphics settings from {}filename{}",
                korangar_debug::MAGENTA,
                korangar_debug::NONE
            );

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        korangar_debug::print_debug!(
            "loading graphics settings from {}filename{}",
            korangar_debug::MAGENTA,
            korangar_debug::NONE
        );

        std::fs::read_to_string("client/graphics_settings.ron")
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        korangar_debug::print_debug!(
            "saving graphics settings to {}filename{}",
            korangar_debug::MAGENTA,
            korangar_debug::NONE
        );

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write("client/graphics_settings.ron", data).expect("unable to write file");
    }
}

impl Drop for GraphicsSettings {
    fn drop(&mut self) {
        self.save();
    }
}
