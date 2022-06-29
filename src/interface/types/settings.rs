use serde::{ Serialize, Deserialize };

#[cfg(feature = "debug")]
use debug::*;
use interface::types::*;

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct InterfaceSettings {
    pub scaling: MutableRange<f32, RERESOLVE>,
    #[hidden_element]
    pub theme_file: String,
}

impl Default for InterfaceSettings {
    
    fn default() -> Self {
        let scaling = MutableRange::new(1.0, 0.7, 1.7);
        let theme_file = "client/themes/theme.json".to_string();
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

        std::fs::read_to_string("client/interface_settings.json")
            .ok()
            .map(|data| serde_json::from_str(&data).ok())
            .flatten()
    }
    
    pub fn save(&self) {

        #[cfg(feature = "debug")]
        print_debug!("saving interface settings to {}filename{}", MAGENTA, NONE);

        let data = serde_json::to_string_pretty(&self).unwrap();
        std::fs::write("client/interface_settings.json", data).expect("unable to write file");
    }
}

impl Drop for InterfaceSettings {

    fn drop(&mut self) {
        self.save();
    }
}
