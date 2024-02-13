use procedural::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::interface::*;

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct InterfaceSettings {
    #[name("Scaling")]
    pub scaling: MutableRange<f32, Reresolve>,
    #[hidden_element]
    pub theme_file: String,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        let scaling = MutableRange::new(1.0, 0.7, 1.7);
        let theme_file = "client/themes/theme.ron".to_string();
        Self { scaling, theme_file }
    }
}

impl InterfaceSettings {
    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load interface settings from {}filename{}", MAGENTA, NONE);

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading interface settings from {}filename{}", MAGENTA, NONE);

        std::fs::read_to_string("client/interface_settings.ron")
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving interface settings to {}filename{}", MAGENTA, NONE);

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();
        std::fs::write("client/interface_settings.ron", data).expect("unable to write file");
    }
}

impl Drop for InterfaceSettings {
    fn drop(&mut self) {
        self.save();
    }
}
