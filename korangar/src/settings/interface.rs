#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::loaders::Scaling;
use crate::state::localization::Language;

/// This theme name includes a zero byte so that it can not point to an actual
/// file. This is guaranteed to fail to load which will automatically fall back
/// to the default theme. The only issue is that loading the default theme will
/// cause an error to appear when running Korangar with debug features.
pub const DEFAULT_THEME_NAME: &str = "^000001default^000000\0";
pub const MENU_THEMES_PATH: &str = "client/menu_themes";
pub const IN_GAME_THEMES_PATH: &str = "client/in_game_themes";
pub const WORLD_THEMES_PATH: &str = "client/world_themes";

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct InterfaceSettings {
    pub language: Language,
    pub scaling: Scaling,
    pub menu_theme: String,
    pub in_game_theme: String,
    pub world_theme: String,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            language: Language::English,
            scaling: Scaling::new(1.0),
            menu_theme: DEFAULT_THEME_NAME.to_string(),
            in_game_theme: DEFAULT_THEME_NAME.to_string(),
            world_theme: DEFAULT_THEME_NAME.to_string(),
        }
    }
}

impl InterfaceSettings {
    const FILE_NAME: &'static str = "client/interface_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load interface settings from {}", Self::FILE_NAME.magenta());

            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading interface settings from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving interface settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save interface settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }
}

impl Drop for InterfaceSettings {
    fn drop(&mut self) {
        self.save();
    }
}

#[derive(RustState, StateElement)]
pub struct InterfaceSettingsCapabilities {
    languages: Vec<Language>,
    scalings: Vec<Scaling>,
    menu_themes: Vec<String>,
    in_game_themes: Vec<String>,
    world_themes: Vec<String>,
}

impl InterfaceSettingsCapabilities {
    fn load_themes(directory: &str) -> Vec<String> {
        let mut themes = vec![DEFAULT_THEME_NAME.to_string()];

        if let Ok(entries) = std::fs::read_dir(directory) {
            themes.extend(
                entries
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| entry.file_name().to_string_lossy().strip_suffix(".ron").map(ToOwned::to_owned)),
            );

            // Sort themes excluding the default since we always want that to be first.
            themes[1..].sort_unstable();
        }

        themes
    }
}

impl Default for InterfaceSettingsCapabilities {
    fn default() -> Self {
        Self {
            // TODO: Don't hardcode this, load it from the disk instead.
            languages: vec![Language::English, Language::German],
            scalings: vec![
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
            menu_themes: Self::load_themes(MENU_THEMES_PATH),
            in_game_themes: Self::load_themes(IN_GAME_THEMES_PATH),
            world_themes: Self::load_themes(WORLD_THEMES_PATH),
        }
    }
}
