use std::collections::HashMap;

use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::windows::Anchor;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;

#[derive(Serialize, Deserialize, new)]
pub struct WindowState {
    pub anchor: Anchor<InterfaceSettings>,
    pub size: ScreenSize,
}

#[derive(Default, Serialize, Deserialize)]
pub struct WindowCache {
    entries: HashMap<String, WindowState>,
}

impl WindowCache {
    const FILE_NAME: &'static str = "client/window_cache.ron";

    fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading window cache from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string("client/window_cache.ron")
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .map(|entries| Self { entries })
    }

    fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving window cache to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(&self.entries, PrettyConfig::new()).unwrap();
        std::fs::write(Self::FILE_NAME, data).expect("unable to write file");
    }
}

impl korangar_interface::application::WindowCache<InterfaceSettings> for WindowCache {
    fn create() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to load window cache from {}. creating empty cache",
                Self::FILE_NAME.magenta()
            );

            Default::default()
        })
    }

    fn register_window(&mut self, identifier: &str, anchor: Anchor<InterfaceSettings>, size: ScreenSize) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.anchor = anchor;
            entry.size = size;
        } else {
            let entry = WindowState::new(anchor, size);
            self.entries.insert(identifier.to_string(), entry);
        }
    }

    fn update_anchor(&mut self, identifier: &str, anchor: Anchor<InterfaceSettings>) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.anchor = anchor;
        }
    }

    fn update_size(&mut self, identifier: &str, size: ScreenSize) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.size = size;
        }
    }

    fn get_window_state(&self, identifier: &str) -> Option<(Anchor<InterfaceSettings>, ScreenSize)> {
        self.entries.get(identifier).map(|entry| (entry.anchor.clone(), entry.size))
    }
}

impl Drop for WindowCache {
    fn drop(&mut self) {
        self.save();
    }
}
