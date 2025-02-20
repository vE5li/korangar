#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub mute_on_focus_loss: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self { mute_on_focus_loss: true }
    }
}

impl AudioSettings {
    const FILE_NAME: &'static str = "client/audio_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load audio settings from {}", Self::FILE_NAME.magenta());
            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading audio settings from {}", Self::FILE_NAME.magenta());
        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving audio settings to {}", Self::FILE_NAME.magenta());
        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write(Self::FILE_NAME, data).expect("unable to write file");
    }
}

impl Drop for AudioSettings {
    fn drop(&mut self) {
        self.save();
    }
}
