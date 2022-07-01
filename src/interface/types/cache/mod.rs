mod state;

use std::collections::HashMap;
use cgmath::Vector2;
use serde::{ Serialize, Deserialize };
use ron::ser::PrettyConfig;

#[cfg(feature = "debug")]
use debug::*;
use interface::types::{ Position, Size };

use self::state::WindowState;

#[derive(Default, Serialize, Deserialize)]
pub struct WindowCache {
    entries: HashMap<String, WindowState>,
}

impl WindowCache {

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {

            #[cfg(feature = "debug")]
            print_debug!("failed to load window cache from {}filename{}. creating empty cache", MAGENTA, NONE);
            
            Default::default()
        })
    }

    pub fn load() -> Option<Self> {

        #[cfg(feature = "debug")]
        print_debug!("loading window cache from {}filename{}", MAGENTA, NONE);

        std::fs::read_to_string("client/window_cache.ron")
            .ok()
            .map(|data| ron::from_str(&data).ok())
            .flatten()
            .map(|entries| Self { entries })
    }
    
    pub fn save(&self) {

        #[cfg(feature = "debug")]
        print_debug!("saving window cache to {}filename{}", MAGENTA, NONE);

        let data = ron::ser::to_string_pretty(&self.entries, PrettyConfig::new()).unwrap();
        std::fs::write("client/window_cache.ron", data).expect("unable to write file");
    }

    pub fn register_window(&mut self, identifier: &str, position: Position, size: Size) {
        if let Some(entry) = self.entries.get_mut(identifier) {
            entry.position = position; 
            entry.size = size; 
        } else {
            let entry = WindowState::new(position, size);
            self.entries.insert(identifier.to_string(), entry);
        }
    }

    pub fn update_position(&mut self, identifier: &str, position: Vector2<f32>) {
        self.entries.get_mut(identifier).map(|entry| entry.position = position);
    }

    pub fn update_size(&mut self, identifier: &str, size: Size) {
        self.entries.get_mut(identifier).map(|entry| entry.size = size);
    }

    pub fn get_window_state(&self, identifier: &str) -> Option<(Position, Size)> {
        self.entries.get(identifier).map(|entry| (entry.position, entry.size))
    }
}

impl Drop for WindowCache {

    fn drop(&mut self) {
        self.save();
    }
}
