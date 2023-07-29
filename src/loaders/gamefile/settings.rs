use procedural::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "debug")]
use crate::debug::*;

#[derive(Serialize, Deserialize, PrototypeElement)]
pub struct GameFileSettings {
    pub archives: Vec<String>,
}

impl Default for GameFileSettings {
    fn default() -> Self {
        let archives: Vec<String> = Vec::new();
        Self { archives }
    }
}

const FILENAME: &str = "client/game_file_settings.ron";

impl GameFileSettings {
    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            panic!("failed to load game file settings from {}{FILENAME}{}", MAGENTA, NONE);

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading game file settings from {}{FILENAME}{}", MAGENTA, NONE);

        std::fs::read_to_string(FILENAME).ok().and_then(|data| ron::from_str(&data).ok())
    }
}
