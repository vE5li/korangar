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

    pub(super) fn load(remote_enabled: bool) -> Result<Self, String> {
        match std::fs::read_to_string(Self::FILE_NAME) {
            Ok(data) => ron::from_str(&data).map_err(|_| format!("Invalid {}: expected (archives: [\"path\", ...])", Self::FILE_NAME)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::without_settings(remote_enabled)),
            Err(_) => Err(format!("Cannot read {}", Self::FILE_NAME)),
        }
    }

    fn without_settings(remote_enabled: bool) -> Self {
        // A remote install does not imply that proprietary local GRFs exist.
        // Explicit local overlays remain opt-in and retain precedence.
        Self {
            archives: if remote_enabled {
                Vec::new()
            } else {
                Self::DEFAULT_FILES
                    .iter()
                    .filter(|path| std::path::Path::new(path).exists())
                    .map(ToString::to_string)
                    .collect()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_defaults_do_not_require_grfs() {
        assert!(GameArchiveList::without_settings(true).archives.is_empty());
    }
    #[test]
    fn explicit_empty_archive_list_and_local_overlays_are_valid() {
        assert!(ron::from_str::<GameArchiveList>("(archives: [])").unwrap().archives.is_empty());
        assert_eq!(
            ron::from_str::<GameArchiveList>("(archives: [\"archive/\"])").unwrap().archives,
            vec!["archive/"]
        );
    }
}
