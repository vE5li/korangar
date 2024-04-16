use std::collections::HashMap;

use derive_new::new;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenPosition, ScreenSize};

#[derive(Serialize, Deserialize, new)]
pub struct WindowState {
    pub position: ScreenPosition,
    pub size: ScreenSize,
}

#[derive(Default, Serialize, Deserialize)]
pub struct WindowCache {
    entries: HashMap<String, WindowState>,
}

impl WindowCache {
    fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        korangar_debug::print_debug!(
            "loading window cache from {}filename{}",
            korangar_debug::MAGENTA,
            korangar_debug::NONE
        );

        std::fs::read_to_string("client/window_cache.ron")
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .map(|entries| Self { entries })
    }

    fn save(&self) {
        #[cfg(feature = "debug")]
        korangar_debug::print_debug!(
            "saving window cache to {}filename{}",
            korangar_debug::MAGENTA,
            korangar_debug::NONE
        );

        let data = ron::ser::to_string_pretty(&self.entries, PrettyConfig::new()).unwrap();
        std::fs::write("client/window_cache.ron", data).expect("unable to write file");
    }
}

impl korangar_interface::application::WindowCache<InterfaceSettings> for WindowCache {
    fn create() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            korangar_debug::print_debug!(
                "failed to load window cache from {}filename{}. creating empty cache",
                korangar_debug::MAGENTA,
                korangar_debug::NONE
            );

            Default::default()
        })
    }

    fn register_window(&mut self, identifier: &str, position: ScreenPosition, size: ScreenSize) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.position = position;
            entry.size = size;
        } else {
            let entry = WindowState::new(position, size);
            self.entries.insert(identifier.to_string(), entry);
        }
    }

    fn update_position(&mut self, identifier: &str, position: ScreenPosition) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.position = position;
        }
    }

    fn update_size(&mut self, identifier: &str, size: ScreenSize) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.size = size;
        }
    }

    fn get_window_state(&self, identifier: &str) -> Option<(ScreenPosition, ScreenSize)> {
        self.entries.get(identifier).map(|entry| (entry.position, entry.size))
    }
}

impl Drop for WindowCache {
    fn drop(&mut self) {
        self.save();
    }
}
