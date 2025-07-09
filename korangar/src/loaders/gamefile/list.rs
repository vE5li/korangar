#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::element::StateElement;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, RustState, StateElement)]
pub(super) struct GameArchiveList {
    pub archives: Vec<String>,
}

impl Default for GameArchiveList {
    fn default() -> Self {
        Self {
            archives: Self::DEFAULT_FILES.iter().map(ToString::to_string).collect(),
        }
    }
}

impl GameArchiveList {
    const DEFAULT_FILES: &'static [&'static str] = &["data.grf", "rdata.grf", "archive/"];
    const FILE_NAME: &'static str = "client/game_archives.ron";

    pub(super) fn load() -> Self {
        #[cfg(feature = "debug")]
        print_debug!("loading game archive list from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .unwrap_or_else(|| {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to load game archive list from {}; trying with default",
                    "warning".yellow(),
                    Self::FILE_NAME.magenta(),
                );

                GameArchiveList::default()
            })
    }
}
