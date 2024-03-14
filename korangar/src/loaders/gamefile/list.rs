use korangar_procedural::PrototypeElement;
use serde::{Deserialize, Serialize};

#[cfg(feature = "debug")]
use crate::debug::*;

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
        print_debug!("loading game archive list from {}{FILENAME}{}", MAGENTA, NONE);

        std::fs::read_to_string(FILENAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .map(|archives| Self { archives })
            .unwrap_or_else(|| {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}error{}] failed to load game archive list from {}{FILENAME}{}; trying with default",
                    RED,
                    NONE,
                    MAGENTA,
                    NONE
                );

                GameArchiveList::default()
            })
    }
}
