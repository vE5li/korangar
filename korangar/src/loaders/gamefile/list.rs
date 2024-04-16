use korangar_interface::elements::PrototypeElement;
use serde::{Deserialize, Serialize};

const FILENAME: &str = "client/game_archives.ron";
const DEFAULT_FILES: &[&str] = &["data.grf", "rdata.grf", "archive/"];

#[derive(Serialize, Deserialize, PrototypeElement)]
pub(super) struct GameArchiveList {
    pub archives: Vec<String>,
}

impl Default for GameArchiveList {
    fn default() -> Self {
        Self {
            archives: DEFAULT_FILES.iter().map(ToString::to_string).collect(),
        }
    }
}

impl GameArchiveList {
    pub(super) fn load() -> Self {
        #[cfg(feature = "debug")]
        korangar_debug::print_debug!(
            "loading game archive list from {}{FILENAME}{}",
            korangar_debug::MAGENTA,
            korangar_debug::NONE
        );

        std::fs::read_to_string(FILENAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .map(|archives| Self { archives })
            .unwrap_or_else(|| {
                #[cfg(feature = "debug")]
                korangar_debug::print_debug!(
                    "[{}error{}] failed to load game archive list from {}{FILENAME}{}; trying with default",
                    korangar_debug::RED,
                    korangar_debug::NONE,
                    korangar_debug::MAGENTA,
                    korangar_debug::NONE
                );

                GameArchiveList::default()
            })
    }
}
